use axum::{
    extract::{Path, State},
    Extension, Json,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

use npc_cache::{cache_ttl, CacheStrategy};
use npc_types::{
    ActionPlanRequest, ActionPlanResponse, NavigationConfig, NavigationRequest, NavigationResponse,
    RegisterSceneRequest, ScenePositionsResponse,
};

use crate::{error::ApiError, state::AppState};

pub async fn register_scene(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<RegisterSceneRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    debug!(
        "Registering scene {} with {} positions",
        req.scene_id,
        req.positions.len()
    );

    let scene_key = format!("scene:{}:meta", req.scene_id);
    let meta = serde_json::json!({
        "scene_id": req.scene_id,
        "world": req.world,
        "position_count": req.positions.len(),
        "registered_at": chrono::Utc::now().timestamp(),
    });

    state
        .redis
        .set_with_ttl(&scene_key, &meta, cache_ttl::SCENE_DATA_TTL)
        .await?;

    for pos in &req.positions {
        let name_key = format!("scene:{}:name:{}", req.scene_id, pos.name.to_lowercase());
        let pos_json = serde_json::to_string(pos).map_err(|e| {
            ApiError::InternalError(format!("Serialization error: {}", e))
        })?;
        state
            .redis
            .set_with_ttl(&name_key, &pos_json, cache_ttl::SCENE_DATA_TTL)
            .await?;

        for alias in &pos.aliases {
            let alias_key = format!("scene:{}:name:{}", req.scene_id, alias.to_lowercase());
            state
                .redis
                .set_with_ttl(&alias_key, &pos_json, cache_ttl::SCENE_DATA_TTL)
                .await?;
        }
    }

    let all_key = format!("scene:{}:positions", req.scene_id);
    let all_json = serde_json::to_string(&req.positions).map_err(|e| {
        ApiError::InternalError(format!("Serialization error: {}", e))
    })?;
    state
        .redis
        .set_with_ttl(&all_key, &all_json, cache_ttl::SCENE_DATA_TTL)
        .await?;

    info!(
        "Scene {} registered with {} positions",
        req.scene_id,
        req.positions.len()
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "scene_id": req.scene_id,
        "positions_registered": req.positions.len(),
    })))
}

pub async fn get_scene_positions(
    State(state): State<AppState>,
    Path(scene_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<ScenePositionsResponse>, ApiError> {
    let cache_key = format!("scene:{}:positions", scene_id);
    let positions: Vec<npc_types::ScenePosition> = state
        .redis
        .get(&cache_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Scene '{}' not found", scene_id)))?;

    let total = positions.len();

    Ok(Json(ScenePositionsResponse {
        scene_id,
        positions,
        total,
    }))
}

pub async fn navigate_npc(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<NavigationRequest>,
) -> Result<Json<NavigationResponse>, ApiError> {
    debug!("Navigation request for agent {}", agent_id);

    let config_key = format!("npc:{}:nav_config", agent_id);
    let config: NavigationConfig = state
        .redis
        .get(&config_key)
        .await?
        .unwrap_or_default();

    let current_pos = get_current_position(&state, &agent_id).await?;

    let (target_pos, target_name) = resolve_navigation_target(&state, &req, &config).await?;

    let engine = npc_core::MovementEngine::new(config.default_speed);

    if let Some(ref restrictions) = config.restrictions {
        engine
            .validate_movement(&current_pos, &target_pos, Some(restrictions))
            .map_err(ApiError::BadRequest)?;
    }

    let speed = req.speed.unwrap_or(config.default_speed);
    let response = engine.calculate_response(
        &current_pos,
        &target_pos,
        target_name,
        config.mode,
        Some(speed),
    );

    let pos_key = format!("npc:position:{}", agent_id);
    state
        .redis
        .set_with_ttl(&pos_key, &target_pos, CacheStrategy::NPC_POSITION_TTL)
        .await?;

    let state_key = format!("npc:state:{}", agent_id);
    let _ = state.redis.del(&state_key).await;
    let _ = state.memory_cache.remove(&agent_id.to_string());

    {
        let nextjs = state.nextjs_client.clone();
        let aid = agent_id.clone();
        let pos = target_pos.clone();
        tokio::spawn(async move {
            let _ = nextjs.update_npc_position(&aid, &pos).await;
        });
    }

    info!(
        "NPC {} navigated via {} mode to {} (distance: {:.2})",
        agent_id,
        response.navigation_mode,
        response
            .target_name
            .as_deref()
            .unwrap_or(&format!("{:?}", response.to)),
        response.distance.unwrap_or(0.0)
    );

    Ok(Json(response))
}

pub async fn set_navigation_config(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(config): Json<NavigationConfig>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let config_key = format!("npc:{}:nav_config", agent_id);
    state
        .redis
        .set_with_ttl(&config_key, &config, cache_ttl::NAV_CONFIG_TTL)
        .await?;

    debug!(
        "Set navigation config for {}: mode={}",
        agent_id,
        config.mode.as_str()
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "agent_id": agent_id,
        "mode": config.mode.as_str(),
    })))
}

pub async fn get_navigation_config(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<NavigationConfig>, ApiError> {
    let config_key = format!("npc:{}:nav_config", agent_id);
    let config: NavigationConfig = state
        .redis
        .get(&config_key)
        .await?
        .unwrap_or_default();

    Ok(Json(config))
}

pub async fn create_action_plan(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<ActionPlanRequest>,
) -> Result<Json<ActionPlanResponse>, ApiError> {
    // Validate plan before creating
    req.plan.validate().map_err(ApiError::BadRequest)?;

    let plan_id = Uuid::new_v4().to_string();
    let action_count = req.plan.actions.len();

    let total_ms: u64 = req
        .plan
        .actions
        .iter()
        .map(|a| a.duration_ms.unwrap_or(1000))
        .sum();

    let plan_key = format!("npc:{}:action_plan:{}", agent_id, plan_id);
    state
        .redis
        .set_with_ttl(&plan_key, &req.plan, cache_ttl::ACTION_PLAN_TTL)
        .await?;

    info!(
        "Created action plan {} for {}: {} actions, {}ms total",
        plan_id, agent_id, action_count, total_ms
    );

    Ok(Json(ActionPlanResponse {
        plan_id,
        action_count,
        total_estimated_ms: total_ms,
        status: "created".to_string(),
    }))
}

async fn get_current_position(
    state: &AppState,
    agent_id: &str,
) -> Result<npc_types::Position3D, ApiError> {
    let pos_key = format!("npc:position:{}", agent_id);
    if let Ok(Some(pos)) = state.redis.get::<npc_types::Position3D>(&pos_key).await {
        return Ok(pos);
    }

    let agent = npc_db::queries::get_agent_with_position(&state.db_pool, agent_id).await?;
    if agent.has_position() {
        let pos = npc_types::Position3D::new(
            agent.position_x.ok_or_else(|| ApiError::InternalError("Missing position_x".to_string()))?,
            agent.position_y.ok_or_else(|| ApiError::InternalError("Missing position_y".to_string()))?,
            agent.position_z.ok_or_else(|| ApiError::InternalError("Missing position_z".to_string()))?,
            agent.world.unwrap_or_else(|| "overworld".to_string()),
        );
        let _ = state
            .redis
            .set_with_ttl(&pos_key, &pos, CacheStrategy::NPC_POSITION_TTL)
            .await;
        return Ok(pos);
    }

    Ok(npc_types::Position3D::new(0.0, 0.0, 0.0, "default".to_string()))
}

async fn resolve_navigation_target(
    state: &AppState,
    req: &NavigationRequest,
    config: &NavigationConfig,
) -> Result<(npc_types::Position3D, Option<String>), ApiError> {
    match &req.target {
        npc_types::NavigationTarget::Coordinate { position } => {
            Ok((position.clone(), None))
        }
        npc_types::NavigationTarget::Relative { dx, dy, dz } => {
            let current = get_current_position(state, &String::new()).await.unwrap_or_else(|_| {
                npc_types::Position3D::new(0.0, 0.0, 0.0, "default".to_string())
            });
            let target = npc_types::Position3D::new(
                current.x + dx,
                current.y + dy,
                current.z + dz,
                current.world.clone(),
            );
            Ok((target, None))
        }
        npc_types::NavigationTarget::Word { name } => {
            let scene_id = config.scene_id.as_deref().unwrap_or("default");
            let name_key = format!("scene:{}:name:{}", scene_id, name.to_lowercase());

            match state.redis.get::<npc_types::ScenePosition>(&name_key).await {
                Ok(Some(scene_pos)) => {
                    Ok((scene_pos.position.clone(), Some(scene_pos.name.clone())))
                }
                _ => {
                    warn!(
                        "Word '{}' not found in scene '{}'",
                        name, scene_id
                    );
                    Err(ApiError::NotFound(format!(
                        "Position '{}' not found in scene '{}'",
                        name, scene_id
                    )))
                }
            }
        }
    }
}
