use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware,
    middleware::Next,
    response::Response,
    routing::{get, post},
    Router,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use tracing::warn;

use crate::{
    handlers,
    middleware::auth_middleware,
    state::AppState,
};

pub fn create_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/health", get(handlers::health_check))
        .route("/metrics", get(handlers::metrics));

    let protected_routes = Router::new()
        .route("/npc/:id", get(handlers::get_npc_state))
        .route("/npc/nearby", get(handlers::get_nearby_npcs))
        .route("/npc/batch-state", post(handlers::batch_get_states))
        .route("/npc/:id/move", post(handlers::move_npc))
        .route("/npc/:id/chat", post(handlers::chat))
        .route("/npc/:id/pathfind", post(handlers::pathfind))
        .route("/npc/:id/navigate", post(handlers::navigate_npc))
        .route("/npc/:id/navigation-config", get(handlers::get_navigation_config))
        .route("/npc/:id/navigation-config", post(handlers::set_navigation_config))
        .route("/npc/:id/action-plan", post(handlers::create_action_plan))
        .route("/npc/:id/action-config", get(handlers::get_action_system_config))
        .route("/npc/:id/action-config", post(handlers::set_action_system_config))
        .route("/npc/:id/ai-context", get(handlers::get_ai_context))
        .route("/npc/:id/action-execution", post(handlers::record_action_execution))
        .route("/npc/:id/action-analytics", get(handlers::get_action_analytics))
        .route("/npc/:id/action-history", get(handlers::get_action_history))
        .route("/navigation/scene/:scene_id/positions", get(handlers::get_scene_positions))
        .route("/navigation/scene/register", post(handlers::register_scene))
        .route("/actions/catalog", get(handlers::get_game_catalog))
        .route("/actions/catalog/:genre", get(handlers::get_genre_default_actions))
        .route("/llm/presets", get(handlers::get_llm_presets))
        .route("/llm/providers", get(handlers::list_llm_providers))
        .route("/llm/providers", post(handlers::register_llm_provider))
        .route("/llm/providers/:name", get(handlers::get_llm_provider_status))
        .route("/llm/providers/:name", post(handlers::unregister_llm_provider))
        .route("/llm/metrics/global", get(handlers::get_request_metrics_global))
        .route("/llm/metrics/agent/:agent_id", get(handlers::get_request_metrics_by_agent))
        .route("/llm/metrics/user/:user_id", get(handlers::get_request_metrics_by_user))
        .route("/llm/metrics/top-agents", get(handlers::get_request_metrics_top_agents))
        .route("/llm/metrics/recent", get(handlers::get_request_metrics_recent))
        .route("/llm/chat/stream", post(handlers::chat_stream))
        .route("/dialogue/ambient", post(handlers::ambient_dialogue))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let webhook_routes = Router::new()
        .route("/webhook/invalidate", post(handlers::invalidate_cache))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            webhook_auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(webhook_routes)
        .with_state(state)
}

/// HMAC-SHA256 webhook authentication middleware
///
/// Expected header: X-Webhook-Signature: <hex-encoded-hmac-sha256>
/// The HMAC is computed over the raw request body
async fn webhook_auth_middleware(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    use axum::body::to_bytes;

    let signature_header = req
        .headers()
        .get("X-Webhook-Signature")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let (parts, body) = req.into_parts();
    let body_bytes = to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;

    match signature_header {
        Some(provided_sig) => {
            let provided_bytes = hex::decode(&provided_sig)
                .map_err(|_| {
                    warn!("Invalid hex signature format");
                    StatusCode::UNAUTHORIZED
                })?;

            let webhook_secret = state.webhook_secret.clone();

            let mut mac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            mac.update(&body_bytes);

            if mac.verify_slice(&provided_bytes).is_ok() {
                let req = Request::from_parts(parts, Body::from(body_bytes));
                Ok(next.run(req).await)
            } else {
                warn!("HMAC verification failed for webhook");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => {
            let jwt_header = parts
                .headers
                .get("Authorization")
                .and_then(|h| h.to_str().ok());

            if let Some(token) = jwt_header.and_then(|h| h.strip_prefix("Bearer ")) {
                let claims = jsonwebtoken::decode::<serde_json::Value>(
                    token,
                    &jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes()),
                    &jsonwebtoken::Validation::default(),
                );

                if claims.is_ok() {
                    let req = Request::from_parts(parts, Body::from(body_bytes));
                    return Ok(next.run(req).await);
                }
            }

            warn!("No valid authentication for webhook");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
