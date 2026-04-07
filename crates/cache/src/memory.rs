use lru::LruCache;
use parking_lot::RwLock;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Thread-safe LRU in-memory cache using parking_lot::RwLock
///
/// parking_lot::RwLock is 2-5x faster than tokio::sync::RwLock for short
/// critical sections because it avoids async-aware lock machinery overhead.
/// Since LRU cache operations (get, put, peek) never .await, this is safe.
pub struct MemoryCache<K, V>
where
    K: Eq + Hash,
{
    cache: Arc<RwLock<LruCache<K, V>>>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Eq + Hash,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("Capacity must be non-zero"),
            ))),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();
        cache.get(key).cloned()
    }

    pub fn peek(&self, key: &K) -> Option<V> {
        let cache = self.cache.read();
        cache.peek(key).cloned()
    }

    pub fn put(&self, key: K, value: V) {
        let mut cache = self.cache.write();
        cache.put(key, value);
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();
        cache.pop(key)
    }

    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    pub fn len(&self) -> usize {
        let cache = self.cache.read();
        cache.len()
    }

    pub fn is_empty(&self) -> bool {
        let cache = self.cache.read();
        cache.is_empty()
    }
}

impl<K, V> Clone for MemoryCache<K, V>
where
    K: Eq + Hash,
{
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_cache_basic_operations() {
        let cache = MemoryCache::new(2);

        cache.put("key1".to_string(), "value1".to_string());
        cache.put("key2".to_string(), "value2".to_string());

        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));
        assert_eq!(cache.get(&"key2".to_string()), Some("value2".to_string()));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_memory_cache_eviction() {
        let cache = MemoryCache::new(2);

        cache.put("key1".to_string(), "value1".to_string());
        cache.put("key2".to_string(), "value2".to_string());
        cache.put("key3".to_string(), "value3".to_string());

        assert_eq!(cache.get(&"key1".to_string()), None);
        assert_eq!(cache.get(&"key3".to_string()), Some("value3".to_string()));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_memory_cache_remove() {
        let cache = MemoryCache::new(5);

        cache.put("key1".to_string(), "value1".to_string());
        assert_eq!(
            cache.remove(&"key1".to_string()),
            Some("value1".to_string())
        );
        assert_eq!(cache.get(&"key1".to_string()), None);
    }

    #[test]
    fn test_memory_cache_clear() {
        let cache = MemoryCache::new(5);

        cache.put("key1".to_string(), "value1".to_string());
        cache.put("key2".to_string(), "value2".to_string());

        cache.clear();
        assert!(cache.is_empty());
    }
}
