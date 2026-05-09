use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub data: T,
    pub expires_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Utc::now() + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[derive(Debug)]
pub struct InMemoryCache<K, V> {
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Duration,
}

impl<K, V> InMemoryCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone,
{
    #[must_use]
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub async fn insert(&self, key: K, value: V) {
        let entry = CacheEntry::new(value, self.default_ttl);
        let mut entries = self.entries.write().await;
        entries.insert(key, entry);
    }

    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let entry = CacheEntry::new(value, ttl);
        let mut entries = self.entries.write().await;
        entries.insert(key, entry);
    }

    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        entries.remove(key).map(|entry| entry.data)
    }

    pub async fn cleanup_expired(&self) {
        let mut entries = self.entries.write().await;
        let now = Utc::now();
        entries.retain(|_, entry| entry.expires_at > now);
    }

    pub async fn size(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn insert_and_get_returns_value() {
        let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::minutes(5));
        cache.insert("key1".to_string(), "hello".to_string()).await;
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some("hello".to_string()));
    }

    #[tokio::test]
    async fn expired_entry_returns_none() {
        let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::minutes(5));
        // Insert with a 1ms TTL. InMemoryCache uses chrono::Utc::now() for
        // expiry tracking (not tokio::time::Instant), so tokio::time::pause
        // cannot mock it. Use a conservative 100ms wall-clock sleep instead
        // to stay reliable on loaded CI runners.
        cache
            .insert_with_ttl(
                "key_exp".to_string(),
                "value".to_string(),
                Duration::milliseconds(1),
            )
            .await;
        tokio::time::sleep(StdDuration::from_millis(100)).await;
        let result = cache.get(&"key_exp".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn cleanup_removes_expired() {
        let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::minutes(5));
        cache
            .insert_with_ttl(
                "stale".to_string(),
                "data".to_string(),
                Duration::milliseconds(1),
            )
            .await;
        // Use a conservative 100ms wall-clock sleep (see expired_entry_returns_none
        // for rationale — chrono-based expiry cannot be mocked via tokio::time::pause).
        tokio::time::sleep(StdDuration::from_millis(100)).await;
        cache.cleanup_expired().await;
        assert_eq!(cache.size().await, 0);
    }

    #[tokio::test]
    async fn clear_empties_cache() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::minutes(5));
        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;
        cache.insert("c".to_string(), 3).await;
        assert_eq!(cache.size().await, 3);
        cache.clear().await;
        assert_eq!(cache.size().await, 0);
    }

    #[tokio::test]
    async fn size_reflects_inserts() {
        let cache: InMemoryCache<u32, &str> = InMemoryCache::new(Duration::minutes(5));
        for i in 0..7u32 {
            cache.insert(i, "v").await;
        }
        assert_eq!(cache.size().await, 7);
        // Re-inserting an existing key must not inflate the count.
        cache.insert(0u32, "updated").await;
        assert_eq!(cache.size().await, 7);
    }
}
