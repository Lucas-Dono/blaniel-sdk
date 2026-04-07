use sqlx::PgPool;
use std::sync::Arc;

use npc_cache::{MemoryCache, RedisCache};
use npc_core::{LLMProviderRegistry, NextJsClient, RequestRouter};
use npc_types::{ChatResponse, NpcState};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub redis: RedisCache,
    pub memory_cache: MemoryCache<String, NpcState>,
    pub chat_memory_cache: MemoryCache<String, ChatResponse>,
    pub nextjs_client: NextJsClient,
    pub request_router: Arc<RequestRouter>,
    pub llm_registry: Arc<LLMProviderRegistry>,
    pub jwt_secret: String,
    pub webhook_secret: String,
}

impl AppState {
    pub fn new(
        db_pool: PgPool,
        redis: RedisCache,
        nextjs_client: NextJsClient,
        llm_registry: LLMProviderRegistry,
        jwt_secret: String,
    ) -> Self {
        let memory_cache_size = std::env::var("MEMORY_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        let memory_cache = MemoryCache::new(memory_cache_size);
        let chat_memory_cache = MemoryCache::new(memory_cache_size);

        let llm_arc = Arc::new(llm_registry);

        let request_router = Arc::new(RequestRouter::new(
            chat_memory_cache.clone(),
            redis.clone(),
            nextjs_client.clone(),
            llm_arc.clone(),
        ));

        let webhook_secret = std::env::var("WEBHOOK_SECRET").unwrap_or_else(|_| jwt_secret.clone());

        Self {
            db_pool,
            redis,
            memory_cache,
            chat_memory_cache,
            nextjs_client,
            request_router,
            llm_registry: llm_arc,
            jwt_secret,
            webhook_secret,
        }
    }
}
