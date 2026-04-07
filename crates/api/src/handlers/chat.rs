use axum::{extract::{Path, State}, Extension, Json};
use tracing::{debug, info};
use validator::Validate;

use npc_types::{ChatRequest, ChatResponse};

use crate::{error::ApiError, state::AppState};

pub async fn chat(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(user_id): Extension<String>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ApiError> {
    debug!("Chat request for agent {} from user {}", agent_id, user_id);

    req.validate()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    info!("Processing chat: '{}'", req.message);

    let ai_context = {
        let context_key = format!("npc:{}:ai_context", agent_id);
        state
            .redis
            .get::<String>(&context_key)
            .await
            .ok()
            .flatten()
    };

    let response = state
        .request_router
        .handle_chat(
            &agent_id,
            &user_id,
            &req.message,
            req.context.unwrap_or_default(),
            ai_context,
        )
        .await?;

    info!(
        "Chat response generated (source: {}, latency: {}ms)",
        response.source, response.latency_ms
    );

    Ok(Json(response))
}
