use axum::{extract::State, Json};
use tracing::{debug, info, warn};

use npc_types::InvalidationRequest;

use crate::{error::ApiError, state::AppState};

pub async fn invalidate_cache(
    State(state): State<AppState>,
    Json(req): Json<InvalidationRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(
        "Cache invalidation request: event_type={}, agent_id={}",
        req.event_type, req.agent_id
    );

    let now = chrono::Utc::now().timestamp();
    if (now - req.timestamp).abs() > 300 {
        warn!("Stale invalidation request rejected (age: {}s)", now - req.timestamp);
        return Err(ApiError::BadRequest("Stale invalidation request".to_string()));
    }

    match req.event_type.as_str() {
        "emotional_update" => {
            invalidate_emotional_state(&state, &req.agent_id).await?;
        }
        "position_update" => {
            invalidate_position(&state, &req.agent_id).await?;
        }
        "personality_change" => {
            invalidate_all(&state, &req.agent_id).await?;
        }
        _ => {
            warn!("Unknown invalidation event type: {}", req.event_type);
            return Err(ApiError::BadRequest(format!(
                "Unknown event type: {}",
                req.event_type
            )));
        }
    }

    info!("Cache invalidated for agent {}: event={}", req.agent_id, req.event_type);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "agent_id": req.agent_id,
        "event_type": req.event_type,
        "invalidated_at": now,
    })))
}

async fn invalidate_emotional_state(
    state: &AppState,
    agent_id: &str,
) -> Result<(), ApiError> {
    let emotions_key = format!("npc:emotions:{}", agent_id);
    state.redis.del(&emotions_key).await?;

    let state_key = format!("npc:state:{}", agent_id);
    let _ = state.redis.del(&state_key).await;
    let _ = state.memory_cache.remove(&agent_id.to_string());

    let chat_count = state.redis.invalidate_chat_cache(agent_id).await?;
    debug!(
        "Invalidated emotional state for {}: {} chat caches cleared",
        agent_id, chat_count
    );

    Ok(())
}

async fn invalidate_position(
    state: &AppState,
    agent_id: &str,
) -> Result<(), ApiError> {
    let position_key = format!("npc:position:{}", agent_id);
    state.redis.del(&position_key).await?;

    let state_key = format!("npc:state:{}", agent_id);
    let _ = state.redis.del(&state_key).await;
    let _ = state.memory_cache.remove(&agent_id.to_string());

    debug!("Invalidated position cache for {}", agent_id);
    Ok(())
}

async fn invalidate_all(
    state: &AppState,
    agent_id: &str,
) -> Result<(), ApiError> {
    state.redis.invalidate_npc_state(agent_id).await?;

    let _ = state.memory_cache.remove(&agent_id.to_string());

    let chat_count = state.redis.invalidate_chat_cache(agent_id).await?;
    debug!(
        "Full cache invalidation for {}: {} chat caches cleared",
        agent_id, chat_count
    );

    Ok(())
}
