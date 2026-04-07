use axum::{
    extract::{Path, State},
    Extension, Json,
};
use tracing::info;
use uuid::Uuid;

use npc_cache::cache_ttl;
use npc_types::{
    ActionAnalytics, ActionExecution, GetActionAnalyticsResponse, RecordActionExecutionRequest,
    RecordActionExecutionResponse,
};

use crate::{error::ApiError, state::AppState};

pub async fn record_action_execution(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
    Json(req): Json<RecordActionExecutionRequest>,
) -> Result<Json<RecordActionExecutionResponse>, ApiError> {
    let execution_id = Uuid::new_v4().to_string();
    let execution = req.execution;

    // Store execution in Redis
    let execution_key = format!(
        "action_history:{}:{}",
        agent_id,
        execution.timestamp
    );
    state
        .redis
        .set_with_ttl(&execution_key, &execution, cache_ttl::ACTION_HISTORY_TTL)
        .await?;

    // Update action counters
    let counter_key = format!("action_stats:{}:{}", agent_id, execution.action_name);
    state.redis.incr(&counter_key).await?;

    // Update success/failure counters
    let result_key = if execution.success {
        format!("action_success:{}:{}", agent_id, execution.action_name)
    } else {
        format!("action_failure:{}:{}", agent_id, execution.action_name)
    };
    state.redis.incr(&result_key).await?;

    info!(
        "Recorded action execution for {}: {} ({}) - {}",
        agent_id,
        execution.action_name,
        execution.action_type.as_str(),
        if execution.success { "success" } else { "failed" }
    );

    Ok(Json(RecordActionExecutionResponse {
        status: "recorded".into(),
        execution_id,
    }))
}

pub async fn get_action_analytics(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<GetActionAnalyticsResponse>, ApiError> {
    let mut analytics = ActionAnalytics::new(agent_id.clone());

    // Get all action names from stats
    let stats_pattern = format!("action_stats:{}:*", agent_id);
    let stats_keys: Vec<String> = state
        .redis
        .scan_keys(&stats_pattern)
        .await
        .unwrap_or_default();

    let mut total_successes = 0u64;
    let mut total_failures = 0u64;

    for key in stats_keys {
        let parts: Vec<&str> = key.split(':').collect();
        if let Some(action_name) = parts.get(2) {
            let count: u64 = state.redis.get(&key).await?.unwrap_or(0);
            analytics
                .action_distribution
                .insert(action_name.to_string(), count);

            // Get success/failure counts
            let success_key = format!("action_success:{}:{}", agent_id, action_name);
            let failure_key = format!("action_failure:{}:{}", agent_id, action_name);

            let successes: u64 = state.redis.get(&success_key).await?.unwrap_or(0);
            let failures: u64 = state.redis.get(&failure_key).await?.unwrap_or(0);

            if successes > 0 {
                analytics
                    .action_success_counts
                    .insert(action_name.to_string(), successes);
                total_successes += successes;
            }

            if failures > 0 {
                analytics
                    .action_failure_counts
                    .insert(action_name.to_string(), failures);
                total_failures += failures;
            }
        }
    }

    analytics.total_actions = total_successes + total_failures;

    // Calculate most used action
    analytics.most_used_action = analytics
        .action_distribution
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(name, _)| name.clone());

    // Calculate success rate
    analytics.success_rate = if analytics.total_actions > 0 {
        (total_successes as f64 / analytics.total_actions as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(GetActionAnalyticsResponse {
        analytics,
        time_range_hours: None,
    }))
}

pub async fn get_action_history(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Extension(_user_id): Extension<String>,
) -> Result<Json<Vec<ActionExecution>>, ApiError> {
    let history_pattern = format!("action_history:{}:*", agent_id);
    let history_keys: Vec<String> = state
        .redis
        .scan_keys(&history_pattern)
        .await
        .unwrap_or_default();

    let mut executions = Vec::new();

    for key in history_keys {
        if let Ok(Some(execution)) = state.redis.get::<ActionExecution>(&key).await {
            executions.push(execution);
        }
    }

    // Sort by timestamp descending (most recent first)
    executions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Limit to 100 most recent
    executions.truncate(100);

    Ok(Json(executions))
}
