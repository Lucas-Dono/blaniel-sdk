pub mod constants;
pub mod memory;
pub mod redis_cache;
pub mod strategies;

pub use constants::cache_ttl;
pub use memory::MemoryCache;
pub use redis_cache::RedisCache;
pub use strategies::CacheStrategy;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache miss")]
    Miss,

    #[error("Invalid key: {0}")]
    InvalidKey(String),
}

pub type CacheResult<T> = Result<T, CacheError>;
