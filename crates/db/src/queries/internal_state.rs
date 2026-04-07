use sqlx::PgPool;
use tracing::debug;

use crate::{models::InternalStateRecord, DbError, DbResult};

/// Get internal emotional state for an agent
pub async fn get_internal_state(pool: &PgPool, agent_id: &str) -> DbResult<InternalStateRecord> {
    debug!("Fetching internal state for agent: {}", agent_id);

    let state = sqlx::query_as::<_, InternalStateRecord>(
        r#"
        SELECT
            id,
            "agentId",
            "currentEmotions",
            "moodValence",
            "moodArousal",
            "moodDominance",
            "lastUpdated"
        FROM "InternalState"
        WHERE "agentId" = $1
        "#,
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Internal state not found for agent: {}", agent_id)))?;

    Ok(state)
}

/// Get internal states for multiple agents (batch query)
pub async fn get_internal_states_batch(
    pool: &PgPool,
    agent_ids: &[String],
) -> DbResult<Vec<InternalStateRecord>> {
    debug!("Batch fetching internal states for {} agents", agent_ids.len());

    let states = sqlx::query_as::<_, InternalStateRecord>(
        r#"
        SELECT
            id,
            "agentId",
            "currentEmotions",
            "moodValence",
            "moodArousal",
            "moodDominance",
            "lastUpdated"
        FROM "InternalState"
        WHERE "agentId" = ANY($1)
        "#,
    )
    .bind(agent_ids)
    .fetch_all(pool)
    .await?;

    Ok(states)
}
