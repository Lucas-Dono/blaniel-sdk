use axum::{extract::State, Json};

use npc_types::HealthResponse;

use crate::state::AppState;

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// GET /api/v1/metrics
pub async fn metrics(State(state): State<AppState>) -> String {
    let db_connected: bool = match state.db_pool.acquire().await {
        Ok(_) => true,
        Err(_) => false,
    };
    let redis_connected: bool = state.redis.exists("nonexistent:key:test").await.is_ok();

    let mem_cache_size = state.memory_cache.len();
    let chat_cache_size = state.chat_memory_cache.len();

    format!(
        "# HELP npc_api_info NPC API version info\n\
         # TYPE npc_api_info gauge\n\
         npc_api_info{{version=\"{}\"}} 1\n\
         # HELP npc_api_db_connected Database connection status\n\
         # TYPE npc_api_db_connected gauge\n\
         npc_api_db_connected{{}} {}\n\
         # HELP npc_api_redis_connected Redis connection status\n\
         # TYPE npc_api_redis_connected gauge\n\
         npc_api_redis_connected{{}} {}\n\
         # HELP npc_api_memory_cache_size Memory cache entries\n\
         # TYPE npc_api_memory_cache_size gauge\n\
         npc_api_memory_cache_size{{type=\"npc\"}} {}\n\
         npc_api_memory_cache_size{{type=\"chat\"}} {}\n",
        env!("CARGO_PKG_VERSION"),
        db_connected as u8,
        redis_connected as u8,
        mem_cache_size,
        chat_cache_size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.0.status, "healthy");
        assert_eq!(response.0.version, env!("CARGO_PKG_VERSION"));
    }
}
