use sqlx::PgPool;
use tracing::debug;

use crate::{models::*, DbError, DbResult};

/// Get agent by ID (basic info)
pub async fn get_agent_by_id(pool: &PgPool, agent_id: &str) -> DbResult<AgentRecord> {
    debug!("Fetching agent by ID: {}", agent_id);

    let agent = sqlx::query_as::<_, AgentRecord>(
        r#"
        SELECT
            id,
            name,
            kind,
            description,
            personality,
            "systemPrompt" as system_prompt,
            metadata,
            "createdAt" as created_at
        FROM "Agent"
        WHERE id = $1
        "#,
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Agent not found: {}", agent_id)))?;

    Ok(agent)
}

/// Get agent with position info from metadata
pub async fn get_agent_with_position(pool: &PgPool, agent_id: &str) -> DbResult<AgentWithPosition> {
    debug!("Fetching agent with position: {}", agent_id);

    let agent = sqlx::query_as::<_, AgentWithPosition>(
        r#"
        SELECT
            id,
            name,
            kind,
            (metadata->>'position_x')::float8 as position_x,
            (metadata->>'position_y')::float8 as position_y,
            (metadata->>'position_z')::float8 as position_z,
            metadata->>'world' as world,
            metadata->>'current_activity' as current_activity,
            metadata
        FROM "Agent"
        WHERE id = $1
        "#,
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::NotFound(format!("Agent not found: {}", agent_id)))?;

    Ok(agent)
}

/// Get agents in a radius (spatial query)
pub async fn get_agents_in_radius(
    pool: &PgPool,
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
    world: &str,
    limit: Option<usize>,
) -> DbResult<Vec<AgentWithPosition>> {
    debug!(
        "Spatial query: x={}, y={}, z={}, radius={}, world={}",
        x, y, z, radius, world
    );

    let limit = limit.unwrap_or(50).min(100); // Cap at 100

    // Optimized query: compare squared distances instead of sqrt (avoids computing sqrt twice)
    let radius_squared = radius * radius;

    let agents = sqlx::query_as::<_, AgentWithPosition>(
        r#"
        SELECT
            id,
            name,
            kind,
            (metadata->>'position_x')::float8 as position_x,
            (metadata->>'position_y')::float8 as position_y,
            (metadata->>'position_z')::float8 as position_z,
            metadata->>'world' as world,
            metadata->>'current_activity' as current_activity,
            metadata
        FROM "Agent"
        WHERE metadata->>'world' = $4
          AND metadata->>'position_x' IS NOT NULL
          AND metadata->>'position_y' IS NOT NULL
          AND metadata->>'position_z' IS NOT NULL
          AND (
              power((metadata->>'position_x')::float8 - $1, 2) +
              power((metadata->>'position_y')::float8 - $2, 2) +
              power((metadata->>'position_z')::float8 - $3, 2)
          ) <= $5
        ORDER BY (
              power((metadata->>'position_x')::float8 - $1, 2) +
              power((metadata->>'position_y')::float8 - $2, 2) +
              power((metadata->>'position_z')::float8 - $3, 2)
          ) ASC
        LIMIT $6
        "#,
    )
    .bind(x)
    .bind(y)
    .bind(z)
    .bind(world)
    .bind(radius_squared)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    debug!("Found {} agents in radius", agents.len());

    Ok(agents)
}

/// Get multiple agents by IDs (batch query)
pub async fn get_agents_by_ids(
    pool: &PgPool,
    agent_ids: &[String],
) -> DbResult<Vec<AgentWithPosition>> {
    debug!("Batch fetching {} agents", agent_ids.len());

    let agents = sqlx::query_as::<_, AgentWithPosition>(
        r#"
        SELECT
            id,
            name,
            kind,
            (metadata->>'position_x')::float8 as position_x,
            (metadata->>'position_y')::float8 as position_y,
            (metadata->>'position_z')::float8 as position_z,
            metadata->>'world' as world,
            metadata->>'current_activity' as current_activity,
            metadata
        FROM "Agent"
        WHERE id = ANY($1)
        "#,
    )
    .bind(agent_ids)
    .fetch_all(pool)
    .await?;

    Ok(agents)
}

/// Check if agent exists
pub async fn agent_exists(pool: &PgPool, agent_id: &str) -> DbResult<bool> {
    let result = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(SELECT 1 FROM "Agent" WHERE id = $1)
        "#,
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await?;

    Ok(result)
}
