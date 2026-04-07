use axum::{extract::State, Extension, Json};
use tracing::debug;

use npc_cache::CacheStrategy;
use npc_core::AmbientDialogueManager;
use npc_types::{AmbientDialogueQuery, AmbientDialoguesResponse};

use crate::{error::ApiError, state::AppState};

/// GET /api/v1/dialogue/ambient
pub async fn ambient_dialogue(
    State(_state): State<AppState>,
    Extension(_user_id): Extension<String>,
    Json(query): Json<AmbientDialogueQuery>,
) -> Result<Json<AmbientDialoguesResponse>, ApiError> {
    debug!(
        "Ambient dialogue request for {} participants in context '{}'",
        query.participant_ids.len(),
        query.context
    );

    let cache_key = format!("dialogue:ambient:{}", query.hash());

    if let Ok(Some(cached)) = _state.redis.get::<AmbientDialoguesResponse>(&cache_key).await {
        debug!("Cache hit for ambient dialogue: {}", cache_key);
        return Ok(Json(cached));
    }

    let manager = AmbientDialogueManager::new();
    let dialogues = manager.generate_ambient_dialogues(&query.participant_ids, &query.context);

    let response = AmbientDialoguesResponse {
        group_hash: query.hash(),
        dialogues,
        cached: false,
    };

    let _ = _state
        .redis
        .set_with_ttl(&cache_key, &response, CacheStrategy::AMBIENT_DIALOGUE_TTL)
        .await;

    Ok(Json(response))
}
