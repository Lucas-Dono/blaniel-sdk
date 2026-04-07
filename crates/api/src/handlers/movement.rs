use axum::{
    extract::{Path, State},
    Extension, Json,
};
use tracing::{debug, info};

use npc_cache::CacheStrategy;
use npc_types::{PathfindRequest, PathResult};

use crate::{error::ApiError, state::AppState};

/// POST /api/v1/npc/:id/pathfind
pub async fn pathfind(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<PathfindRequest>,
) -> Result<Json<PathResult>, ApiError> {
    debug!(
        "Pathfind request for agent {} from {:?} to {:?}",
        agent_id, req.start, req.goal
    );

    let cache_key = format!(
        "path:{}:{}",
        req.start.hash_key(),
        req.goal.hash_key()
    );

    if let Ok(Some(cached)) = state.redis.get::<PathResult>(&cache_key).await {
        debug!("Cache hit for pathfind: {}", cache_key);
        let mut result = cached;
        result.cached = Some(true);
        return Ok(Json(result));
    }

    let engine = npc_core::PathfindingEngine::new(npc_core::NavigationMesh::new_flat(1000, 64));

    let result = engine.find_path(&req.start, &req.goal, &req.options);

    info!(
        "Path computed: {} waypoints, cost: {:.2}, in {}ms",
        result.path.len(),
        result.cost,
        result.computed_in_ms
    );

    let _ = state
        .redis
        .set_with_ttl(&cache_key, &result, CacheStrategy::PATHFINDING_TTL)
        .await;

    Ok(Json(result))
}
