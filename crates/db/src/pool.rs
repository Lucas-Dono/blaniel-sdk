use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

use crate::DbResult;

/// Create a PostgreSQL connection pool
pub async fn create_pool(database_url: &str) -> DbResult<PgPool> {
    let max_connections = std::env::var("DB_POOL_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    let min_connections = std::env::var("DB_POOL_MIN_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let acquire_timeout = std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    info!(
        "Creating database pool with max_connections={}, min_connections={}",
        max_connections, min_connections
    );

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout))
        .connect(database_url)
        .await?;

    info!("Database pool created successfully");

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires database connection
    async fn test_pool_creation() {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let pool = create_pool(&database_url).await;
        assert!(pool.is_ok());

        let pool = pool.unwrap();
        assert!(pool.acquire().await.is_ok());
    }
}
