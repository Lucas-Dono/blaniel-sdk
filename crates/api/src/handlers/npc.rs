use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use futures::future::join_all;
use tracing::{debug, info};

use npc_cache::CacheStrategy;
use npc_db::queries;
use npc_types::{
    Animation, BatchStateRequest, BatchStateResponse, FacingDirection, NearbyNpcsResponse,
    NpcAction, NpcState, Position3D, ProximityQuery,
};

use crate::{error::ApiError, state::AppState};

/// Helper function to fetch NPC state from two-level cache (memory + Redis) or database
async fn fetch_npc_state_cached(
    state: &AppState,
    agent_id: &str,
) -> Result<NpcState, ApiError> {
    // 1. Check memory cache
    if let Some(cached) = state.memory_cache.get(&agent_id.to_string()) {
        debug!("Memory cache hit for NPC: {}", agent_id);
        return Ok(cached);
    }

    // 2. Check Redis cache
    let cache_key = format!("npc:state:{}", agent_id);
    if let Ok(Some(cached)) = state.redis.get::<NpcState>(&cache_key).await {
        debug!("Redis cache hit for NPC: {}", agent_id);
        state.memory_cache.put(agent_id.to_string(), cached.clone());
        return Ok(cached);
    }

    // 3. Query database
    let agent = queries::get_agent_with_position(&state.db_pool, agent_id).await?;
    let internal_state = queries::get_internal_state(&state.db_pool, agent_id).await;

    let has_pos = agent.has_position();
    let npc_state = NpcState {
        id: agent.id.clone(),
        name: agent.name.clone(),
        position: if has_pos {
            Some(Position3D {
                x: agent.position_x.ok_or_else(|| ApiError::InternalError("Missing position_x despite has_position".to_string()))?,
                y: agent.position_y.ok_or_else(|| ApiError::InternalError("Missing position_y despite has_position".to_string()))?,
                z: agent.position_z.ok_or_else(|| ApiError::InternalError("Missing position_z despite has_position".to_string()))?,
                world: agent.world.unwrap_or_else(|| "overworld".to_string()),
            })
        } else {
            None
        },
        current_action: agent
            .current_activity
            .and_then(|a| NpcAction::from_str(&a))
            .unwrap_or_default(),
        animation: Animation::default(),
        facing_direction: FacingDirection::North,
        cached_emotion: internal_state
            .map(|s| s.get_dominant_emotion())
            .unwrap_or_else(|_| "neutral".to_string()),
        cached_at: chrono::Utc::now().timestamp(),
        metadata: agent.metadata,
    };

    // 4. Cache the result
    let _ = state
        .redis
        .set_with_ttl(&cache_key, &npc_state, CacheStrategy::NPC_STATE_TTL)
        .await;
    state.memory_cache.put(agent_id.to_string(), npc_state.clone());

    Ok(npc_state)
}

/// GET /api/v1/npc/:id
pub async fn get_npc_state(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<NpcState>, ApiError> {
    debug!("Getting NPC state for: {}", agent_id);
    let npc_state = fetch_npc_state_cached(&state, &agent_id).await?;
    info!("NPC state fetched and cached");
    Ok(Json(npc_state))
}

/// GET /api/v1/npc/nearby?x=100&y=64&z=200&radius=50&world=overworld
pub async fn get_nearby_npcs(
    State(state): State<AppState>,
    Query(params): Query<ProximityQuery>,
) -> Result<Json<NearbyNpcsResponse>, ApiError> {
    debug!(
        "Proximity query: ({}, {}, {}) radius={} world={}",
        params.x, params.y, params.z, params.radius, params.world
    );

    // Validate
    params
        .validate()
        .map_err(|e| ApiError::BadRequest(e))?;

    // Query database
    let agents = queries::get_agents_in_radius(
        &state.db_pool,
        params.x,
        params.y,
        params.z,
        params.radius,
        &params.world,
        params.limit,
    )
    .await?;

    // Convert to NpcState
    let npcs: Vec<NpcState> = agents
        .into_iter()
        .map(|agent| {
            let has_pos = agent.has_position();
            NpcState {
                id: agent.id,
                name: agent.name.clone(),
                position: if has_pos {
                    Some(Position3D {
                        x: agent.position_x.unwrap(),
                        y: agent.position_y.unwrap(),
                        z: agent.position_z.unwrap(),
                        world: agent.world.unwrap_or_else(|| "overworld".to_string()),
                    })
                } else {
                    None
                },
                current_action: agent
                    .current_activity
                    .and_then(|a| NpcAction::from_str(&a))
                    .unwrap_or_default(),
                animation: Animation::default(),
                facing_direction: FacingDirection::North,
                cached_emotion: "neutral".to_string(),
                cached_at: chrono::Utc::now().timestamp(),
                metadata: agent.metadata,
            }
        })
        .collect();

    let total = npcs.len();

    Ok(Json(NearbyNpcsResponse { npcs, total }))
}

/// POST /api/v1/npc/batch-state
pub async fn batch_get_states(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<BatchStateRequest>,
) -> Result<Json<BatchStateResponse>, ApiError> {
    req.validate()
        .map_err(|e| ApiError::BadRequest(e))?;

    debug!("Batch fetching {} NPC states", req.agent_ids.len());

    // Fetch in parallel
    let futures = req.agent_ids.iter().map(|id| {
        let state = state.clone();
        let id = id.clone();
        async move {
            match get_single_npc_state(&state, &id).await {
                Ok(npc_state) => Ok(npc_state),
                Err(e) => Err(format!("Failed to fetch {}: {:?}", id, e)),
            }
        }
    });

    let results = join_all(futures).await;

    Ok(Json(BatchStateResponse { states: results }))
}

async fn get_single_npc_state(state: &AppState, agent_id: &str) -> Result<NpcState, ApiError> {
    fetch_npc_state_cached(state, agent_id).await
}

/// POST /api/v1/npc/:id/move
pub async fn move_npc(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<npc_types::MoveRequest>,
) -> Result<StatusCode, ApiError> {
    debug!("Moving NPC {} to {:?}", agent_id, req.position);

    // Update position in Redis (fast write)
    let position_key = format!("npc:position:{}", agent_id);
    state
        .redis
        .set_with_ttl(&position_key, &req.position, CacheStrategy::NPC_POSITION_TTL)
        .await?;

    // Invalidate state cache
    let state_key = format!("npc:state:{}", agent_id);
    let _ = state.redis.del(&state_key).await;
    let _ = state.memory_cache.remove(&agent_id);

    // Async update to database via Next.js
    let nextjs_client = state.nextjs_client.clone();
    let agent_id_clone = agent_id.clone();
    let position = req.position.clone();

    tokio::spawn(async move {
        let _ = nextjs_client
            .update_npc_position(&agent_id_clone, &position)
            .await;
    });

    Ok(StatusCode::NO_CONTENT)
}
