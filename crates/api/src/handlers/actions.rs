use axum::{
    extract::{Path, State},
    Extension, Json,
};
use tracing::info;

use npc_cache::{cache_ttl, CacheStrategy};
use npc_types::{
    ActionSystemConfig, GameCatalogResponse, SetActionSystemConfigRequest,
};

use crate::{error::ApiError, state::AppState};

pub async fn get_game_catalog(
    State(_state): State<AppState>,
    Extension(_user_id): Extension<String>,
) -> Json<GameCatalogResponse> {
    Json(GameCatalogResponse::build_catalog())
}

pub async fn set_action_system_config(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<SetActionSystemConfigRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.introduction.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "The 'introduction' field is required and cannot be empty".into(),
        ));
    }
    if req.action_explain.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "The 'action_explain' field is required and cannot be empty".into(),
        ));
    }
    if req.movement_explain.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "The 'movement_explain' field is required and cannot be empty".into(),
        ));
    }

    let config = npc_types::ActionSystemConfig::new(
        req.game_genre.clone(),
        req.enabled_actions,
        req.custom_actions,
        req.introduction,
        req.action_explain,
        req.movement_explain,
    );

    let config_key = format!("npc:{}:action_config", agent_id);
    state
        .redis
        .set_with_ttl(&config_key, &config, cache_ttl::ACTION_CONFIG_TTL)
        .await?;

    let nav_config_key = format!("npc:{}:nav_config", agent_id);
    if let Ok(Some(mut nav_config)) = state
        .redis
        .get::<npc_types::NavigationConfig>(&nav_config_key)
        .await
    {
        nav_config.introduction = config.introduction.clone();
        nav_config.explain = config.movement_explain.clone();
        let _ = state
            .redis
            .set_with_ttl(&nav_config_key, &nav_config, cache_ttl::NAV_CONFIG_TTL)
            .await;
    }

    let mut config_mut = config.clone();
    let ai_context = config_mut.build_ai_context();

    let context_key = format!("npc:{}:ai_context", agent_id);
    state
        .redis
        .set_with_ttl(&context_key, &ai_context, cache_ttl::AI_CONTEXT_TTL)
        .await?;

    info!(
        "Set action system config for {}: genre={}, {} actions enabled, {} custom actions",
        agent_id,
        config.game_genre.as_str(),
        config.enabled_actions.len(),
        config.custom_actions.len()
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "agent_id": agent_id,
        "game_genre": config.game_genre.as_str(),
        "enabled_actions": config.enabled_actions.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
        "custom_actions_count": config.custom_actions.len(),
    })))
}

pub async fn get_action_system_config(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<ActionSystemConfig>, ApiError> {
    let config_key = format!("npc:{}:action_config", agent_id);
    let config: ActionSystemConfig = state
        .redis
        .get(&config_key)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!(
            "Action system config not found for agent '{}'",
            agent_id
        )))?;

    Ok(Json(config))
}

pub async fn get_ai_context(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let context_key = format!("npc:{}:ai_context", agent_id);

    if let Ok(Some(context)) = state.redis.get::<String>(&context_key).await {
        return Ok(Json(serde_json::json!({
            "agent_id": agent_id,
            "context": context,
        })));
    }

    let config_key = format!("npc:{}:action_config", agent_id);
    if let Ok(Some(mut config)) = state.redis.get::<ActionSystemConfig>(&config_key).await {
        let context = config.build_ai_context();
        let _ = state
            .redis
            .set_with_ttl(&context_key, &context, cache_ttl::AI_CONTEXT_TTL)
            .await;

        return Ok(Json(serde_json::json!({
            "agent_id": agent_id,
            "context": context,
        })));
    }

    Err(ApiError::NotFound(format!(
        "No AI context found for agent '{}'",
        agent_id
    )))
}

pub async fn get_genre_default_actions(
    State(_state): State<AppState>,
    Path(genre): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let genre_variant = match genre.as_str() {
        "rpg" => npc_types::GameGenre::Rpg,
        "fighting" => npc_types::GameGenre::Fighting,
        "magic" => npc_types::GameGenre::Magic,
        "adventure" => npc_types::GameGenre::Adventure,
        "romance" => npc_types::GameGenre::Romance,
        "simulation" => npc_types::GameGenre::Simulation,
        "visual_novel" => npc_types::GameGenre::VisualNovel,
        "survival" => npc_types::GameGenre::Survival,
        "horror" => npc_types::GameGenre::Horror,
        "sandbox" => npc_types::GameGenre::Sandbox,
        "strategy" => npc_types::GameGenre::Strategy,
        "sports" => npc_types::GameGenre::Sports,
        "puzzle" => npc_types::GameGenre::Puzzle,
        "platformer" => npc_types::GameGenre::Platformer,
        "shooter" => npc_types::GameGenre::Shooter,
        "stealth" => npc_types::GameGenre::Stealth,
        "racing" => npc_types::GameGenre::Racing,
        "rhythm" => npc_types::GameGenre::Rhythm,
        other => npc_types::GameGenre::Custom(other.to_string()),
    };

    let defaults = genre_variant.default_actions();
    let actions: Vec<serde_json::Value> = defaults
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.as_str(),
                "description": a.description(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "genre": genre,
        "default_actions": actions,
        "total": actions.len(),
    })))
}
