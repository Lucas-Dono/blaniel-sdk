use sqlx::PgPool;
use tracing::debug;

use crate::{models::MessageRecord, DbResult};

/// Get recent messages for an agent-user conversation
pub async fn get_recent_messages(
    pool: &PgPool,
    agent_id: &str,
    user_id: Option<&str>,
    limit: usize,
) -> DbResult<Vec<MessageRecord>> {
    debug!("Fetching recent messages for agent: {}, limit: {}", agent_id, limit);

    let messages = if let Some(uid) = user_id {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT
                id,
                "agentId",
                "userId",
                role,
                content,
                metadata,
                "createdAt"
            FROM "Message"
            WHERE "agentId" = $1 AND "userId" = $2
            ORDER BY "createdAt" DESC
            LIMIT $3
            "#,
        )
        .bind(agent_id)
        .bind(uid)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, MessageRecord>(
            r#"
            SELECT
                id,
                "agentId",
                "userId",
                role,
                content,
                metadata,
                "createdAt"
            FROM "Message"
            WHERE "agentId" = $1
            ORDER BY "createdAt" DESC
            LIMIT $2
            "#,
        )
        .bind(agent_id)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?
    };

    // Reverse to get chronological order
    let mut messages = messages;
    messages.reverse();

    Ok(messages)
}

/// Get message count for an agent
pub async fn get_message_count(pool: &PgPool, agent_id: &str) -> DbResult<i64> {
    let count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) FROM "Message" WHERE "agentId" = $1
        "#,
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await?;

    Ok(count)
}
