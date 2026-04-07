use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
    Extension, Json,
};
use futures::stream::Stream;
use std::convert::Infallible;
use tracing::info;

use npc_types::{
    LLMProviderConfig, LLMProviderStatus, LLMProviderType, LLMProvidersResponse,
    SetLLMProviderRequest,
};

use crate::{error::ApiError, state::AppState};

fn verify_admin(user_id: &str) -> Result<(), ApiError> {
    let admin_ids: Vec<String> = std::env::var("ADMIN_USER_IDS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if admin_ids.is_empty() {
        return Ok(());
    }

    if admin_ids.iter().any(|id| id == user_id) {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

fn validate_base_url(url: &str, allow_local: bool) -> Result<(), ApiError> {
    let parsed = url::Url::parse(url)
        .map_err(|_| ApiError::BadRequest("Invalid base_url format".into()))?;

    let scheme = parsed.scheme();
    if scheme != "https" && scheme != "http" {
        return Err(ApiError::BadRequest(
            "base_url must use http or https".into(),
        ));
    }

    if !allow_local {
        if let Some(host) = parsed.host_str() {
            let is_local = host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host.starts_with("10.")
                || host.starts_with("192.168.")
                || host.starts_with("172.16.")
                || host.starts_with("172.17.")
                || host.starts_with("172.18.")
                || host.starts_with("172.19.")
                || host.starts_with("172.2")
                || host.starts_with("172.3")
                || host.starts_with("0.0.0.0");

            if is_local {
                return Err(ApiError::BadRequest(
                    "Internal/local URLs not allowed for remote providers".into(),
                ));
            }
        }
    }

    Ok(())
}

fn sanitize_header_key(key: &str) -> Result<(), ApiError> {
    if key.is_empty() || key.len() > 100 {
        return Err(ApiError::BadRequest(
            "Header key must be 1-100 characters".into(),
        ));
    }
    if !key.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Err(ApiError::BadRequest(
            "Header key contains invalid characters".into(),
        ));
    }
    Ok(())
}

pub async fn get_llm_presets(
    State(_state): State<AppState>,
    Extension(_user_id): Extension<String>,
) -> Json<LLMProvidersResponse> {
    let presets = LLMProviderType::all_presets().to_vec();
    let total = presets.len();
    Json(LLMProvidersResponse { presets, total })
}

pub async fn list_llm_providers(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
) -> Json<serde_json::Value> {
    let statuses = state.llm_registry.list_providers().await;
    let enabled = statuses.iter().filter(|s| s.enabled).count();
    let healthy = statuses.iter().filter(|s| s.healthy).count();

    Json(serde_json::json!({
        "providers": statuses,
        "total": statuses.len(),
        "enabled": enabled,
        "healthy": healthy,
    }))
}

pub async fn register_llm_provider(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(req): Json<SetLLMProviderRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_admin(&user_id)?;

    let allow_local = matches!(
        req.provider_type,
        LLMProviderType::Ollama | LLMProviderType::LMStudio | LLMProviderType::Custom
    );

    let base_url = req
        .base_url
        .or_else(|| req.provider_type.default_base_url().map(|s| s.to_string()))
        .ok_or_else(|| {
            ApiError::BadRequest("base_url is required for custom providers".into())
        })?;

    validate_base_url(&base_url, allow_local)?;

    if req.max_tokens == 0 || req.max_tokens > 32768 {
        return Err(ApiError::BadRequest(
            "max_tokens must be between 1 and 32768".into(),
        ));
    }

    if req.temperature < 0.0 || req.temperature > 2.0 {
        return Err(ApiError::BadRequest(
            "temperature must be between 0.0 and 2.0".into(),
        ));
    }

    if req.timeout_ms < 1000 || req.timeout_ms > 120000 {
        return Err(ApiError::BadRequest(
            "timeout_ms must be between 1000 and 120000".into(),
        ));
    }

    for key in req.extra_headers.keys() {
        sanitize_header_key(key)?;
    }

    let name = match req.provider_type {
        LLMProviderType::Custom => {
            if req.model.as_ref().map_or(true, |m| m.is_empty()) {
                return Err(ApiError::BadRequest(
                    "model name is required for custom providers".into(),
                ));
            }
            format!("custom_{}", req.model.as_deref().unwrap_or("unknown"))
        }
        other => other
            .default_base_url()
            .map(|_| format!("{:?}", other).to_lowercase())
            .unwrap_or_else(|| "custom".into()),
    };

    let default_model = req
        .provider_type
        .default_base_url()
        .map(|_| "gpt-4o-mini")
        .unwrap_or("unknown");

    let config = LLMProviderConfig {
        name: name.clone(),
        provider_type: req.provider_type,
        base_url,
        api_key: req.api_key,
        model: req.model.unwrap_or_else(|| default_model.into()),
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        system_prompt: req.system_prompt,
        extra_headers: req.extra_headers,
        enabled: req.enabled,
        priority: req.priority,
        timeout_ms: req.timeout_ms,
    };

    state
        .llm_registry
        .register(config)
        .await
        .map_err(ApiError::InternalError)?;

    info!("Registered LLM provider via API: {} by user: {}", name, user_id);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "provider": name,
    })))
}

pub async fn unregister_llm_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(user_id): Extension<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    verify_admin(&user_id)?;

    if state.llm_registry.unregister(&name).await {
        info!("Unregistered LLM provider: {} by user: {}", name, user_id);
        Ok(Json(serde_json::json!({
            "status": "ok",
            "removed": name,
        })))
    } else {
        Err(ApiError::NotFound(format!(
            "Provider '{}' not found",
            name
        )))
    }
}

pub async fn get_llm_provider_status(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<LLMProviderStatus>, ApiError> {
    let client = state
        .llm_registry
        .get(&name)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Provider '{}' not found", name)))?;

    Ok(Json(client.get_status().await))
}

pub async fn get_request_metrics_global(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
) -> Json<serde_json::Value> {
    let summary = state.llm_registry.metrics().global_summary();
    Json(serde_json::to_value(summary).unwrap_or_default())
}

pub async fn get_request_metrics_by_agent(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let summary = state
        .llm_registry
        .metrics()
        .agent_summary(&agent_id)
        .ok_or_else(|| ApiError::NotFound(format!("No metrics for agent '{}'", agent_id)))?;

    Ok(Json(serde_json::to_value(summary).unwrap_or_default()))
}

pub async fn get_request_metrics_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let summary = state
        .llm_registry
        .metrics()
        .user_summary(&user_id)
        .ok_or_else(|| ApiError::NotFound(format!("No metrics for user '{}'", user_id)))?;

    Ok(Json(serde_json::to_value(summary).unwrap_or_default()))
}

pub async fn get_request_metrics_top_agents(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
) -> Json<serde_json::Value> {
    let top = state.llm_registry.metrics().top_agents(10);
    Json(serde_json::json!({ "agents": top }))
}

pub async fn get_request_metrics_recent(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
) -> Json<serde_json::Value> {
    let recent = state.llm_registry.metrics().recent_records(50).await;
    Json(serde_json::json!({ "records": recent, "count": recent.len() }))
}

pub async fn chat_stream(
    State(state): State<AppState>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<npc_types::LLMChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let agent_id = req.messages
        .first()
        .and_then(|m| {
            if m.role == npc_types::LLMRole::System {
                None
            } else {
                Some("stream")
            }
        })
        .unwrap_or("stream");

    let message = req.messages
        .iter()
        .find(|m| m.role == npc_types::LLMRole::User)
        .map(|m| m.content.clone())
        .ok_or_else(|| ApiError::BadRequest("No user message provided".into()))?;

    let system_prompt = req.messages
        .iter()
        .find(|m| m.role == npc_types::LLMRole::System)
        .map(|m| m.content.clone());

    let context = npc_types::ChatContext::default();

    let stream_result = state
        .llm_registry
        .chat_stream(
            agent_id,
            "stream_user",
            &message,
            context,
            system_prompt.as_deref(),
            None,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(32);

    tokio::spawn(async move {
        use tokio_stream::StreamExt;
        let mut stream = stream_result;
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let data = serde_json::to_string(&chunk).unwrap_or_default();
                    let event = Event::default().data(data);
                    if tx.send(Ok(event)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let error_data = serde_json::json!({"error": e.to_string(), "done": true});
                    let event = Event::default().data(error_data.to_string());
                    let _ = tx.send(Ok(event)).await;
                    break;
                }
            }
        }
    });

    Ok(Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
}
