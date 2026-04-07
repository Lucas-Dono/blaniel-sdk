use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, error};

use crate::{CacheError, CacheResult};

#[derive(Clone)]
pub struct RedisCache {
    pub(crate) manager: ConnectionManager,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> CacheResult<Self> {
        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self { manager })
    }

    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> CacheResult<Option<T>> {
        let mut conn = self.manager.clone();

        match conn.get::<_, Option<String>>(key).await {
            Ok(Some(json)) => {
                debug!("Cache hit for key: {}", key);
                let value = serde_json::from_str(&json)?;
                Ok(Some(value))
            }
            Ok(None) => {
                debug!("Cache miss for key: {}", key);
                Ok(None)
            }
            Err(e) => {
                error!("Redis error getting key {}: {}", key, e);
                Err(CacheError::Redis(e))
            }
        }
    }

    pub async fn set_with_ttl<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_seconds: u64,
    ) -> CacheResult<()> {
        let mut conn = self.manager.clone();
        let json = serde_json::to_string(value)?;

        conn.set_ex::<_, _, ()>(key, json, ttl_seconds).await?;
        debug!("Cached key {} with TTL {}s", key, ttl_seconds);

        Ok(())
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> CacheResult<()> {
        let mut conn = self.manager.clone();
        let json = serde_json::to_string(value)?;

        conn.set::<_, _, ()>(key, json).await?;
        debug!("Cached key {} (no TTL)", key);

        Ok(())
    }

    pub async fn del(&self, key: &str) -> CacheResult<()> {
        let mut conn = self.manager.clone();
        conn.del::<_, ()>(key).await?;
        debug!("Deleted key: {}", key);
        Ok(())
    }

    /// Delete multiple keys in a single round-trip
    pub async fn del_many(&self, keys: &[String]) -> CacheResult<()> {
        if keys.is_empty() {
            return Ok(());
        }
        let mut conn = self.manager.clone();
        conn.del::<_, ()>(keys).await?;
        debug!("Deleted {} keys in batch", keys.len());
        Ok(())
    }

    /// Delete keys matching a pattern using SCAN (non-blocking)
    ///
    /// Uses SCAN instead of KEYS to avoid blocking the Redis event loop.
    /// KEYS is O(N) over ALL keys and blocks Redis; SCAN is incremental.
    pub async fn del_pattern(&self, pattern: &str) -> CacheResult<usize> {
        let mut conn = self.manager.clone();

        let mut all_keys: Vec<String> = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) =
                redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await?;

            cursor = new_cursor;
            all_keys.extend(keys);

            if cursor == 0 {
                break;
            }
        }

        if all_keys.is_empty() {
            return Ok(0);
        }

        let count = all_keys.len();
        conn.del::<_, ()>(&all_keys).await?;
        debug!("Deleted {} keys matching pattern: {}", count, pattern);

        Ok(count)
    }

    pub async fn exists(&self, key: &str) -> CacheResult<bool> {
        let mut conn = self.manager.clone();
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    /// Scan for keys matching a pattern without deleting them
    pub async fn scan_keys(&self, pattern: &str) -> CacheResult<Vec<String>> {
        let mut conn = self.manager.clone();

        let mut all_keys: Vec<String> = Vec::new();
        let mut cursor: u64 = 0;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) =
                redis::cmd("SCAN")
                    .arg(cursor)
                    .arg("MATCH")
                    .arg(pattern)
                    .arg("COUNT")
                    .arg(100)
                    .query_async(&mut conn)
                    .await?;

            cursor = new_cursor;
            all_keys.extend(keys);

            if cursor == 0 {
                break;
            }
        }

        Ok(all_keys)
    }

    /// Increment a counter
    pub async fn incr(&self, key: &str) -> CacheResult<i64> {
        let mut conn = self.manager.clone();
        let value: i64 = conn.incr(key, 1).await?;
        Ok(value)
    }

    /// Set multiple key-value pairs in a single Redis pipeline round-trip
    pub async fn set_many_with_ttl<T: Serialize>(
        &self,
        items: &[(String, T, u64)],
    ) -> CacheResult<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut conn = self.manager.clone();
        let mut pipe = redis::pipe();

        for (key, value, ttl) in items {
            let json = serde_json::to_string(value)?;
            pipe.cmd("SETEx").arg(key.as_str()).arg(*ttl).arg(json).ignore();
        }

        pipe.query_async::<_, ()>(&mut conn).await?;
        debug!("Pipelined {} SET operations", items.len());

        Ok(())
    }

    /// Invalidate all cache entries for a specific NPC using batch delete
    pub async fn invalidate_npc_state(&self, agent_id: &str) -> CacheResult<()> {
        let keys = vec![
            format!("npc:state:{}", agent_id),
            format!("npc:emotions:{}", agent_id),
            format!("npc:position:{}", agent_id),
        ];

        self.del_many(&keys).await?;

        debug!("Invalidated all cache for agent: {}", agent_id);
        Ok(())
    }

    pub async fn invalidate_chat_cache(&self, agent_id: &str) -> CacheResult<usize> {
        let pattern = format!("chat:{}:*", agent_id);
        self.del_pattern(&pattern).await
    }

    pub async fn stats(&self) -> CacheResult<CacheStats> {
        let mut conn = self.manager.clone();

        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut conn)
            .await?;

        let hits = info
            .lines()
            .find(|line| line.starts_with("keyspace_hits:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let misses = info
            .lines()
            .find(|line| line.starts_with("keyspace_misses:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);

        let hit_ratio = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };

        Ok(CacheStats {
            hits,
            misses,
            hit_ratio,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_redis_set_get() {
        let cache = RedisCache::new("redis://localhost:6379")
            .await
            .expect("Failed to connect to Redis");

        let key = "test:key";
        let value = "test_value";

        cache.set(key, &value).await.unwrap();
        let result: Option<String> = cache.get(key).await.unwrap();

        assert_eq!(result, Some(value.to_string()));

        cache.del(key).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_ttl() {
        let cache = RedisCache::new("redis://localhost:6379")
            .await
            .expect("Failed to connect to Redis");

        let key = "test:ttl:key";
        let value = "test_value";

        cache.set_with_ttl(key, &value, 1).await.unwrap();

        let exists = cache.exists(key).await.unwrap();
        assert!(exists);

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let exists = cache.exists(key).await.unwrap();
        assert!(!exists);
    }
}
