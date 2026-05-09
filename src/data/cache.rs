use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

#[derive(Debug)]
pub struct CacheEntry<T> {
    pub data: T,
    pub expires_at: Instant,
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
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
        let now = Instant::now();
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

    #[tokio::test]
    async fn test_cache_insert_and_retrieve() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::from_secs(60));
        cache.insert("key".to_string(), "value".to_string()).await;
        let result = cache.get(&"key".to_string()).await;
        assert_eq!(result, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_cache_miss_for_missing_key() {
        let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::from_secs(60));
        let result = cache.get(&"nonexistent".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test(start_paused = true)]
    async fn test_cache_value_expires_after_ttl() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::from_millis(100));
        cache.insert("key".to_string(), "hello".to_string()).await;

        // Value should be present immediately
        assert!(cache.get(&"key".to_string()).await.is_some());

        // Advance mock time past the TTL
        tokio::time::advance(Duration::from_millis(150)).await;

        // Value should now be expired
        let result = cache.get(&"key".to_string()).await;
        assert_eq!(result, None, "cache entry should have expired");
    }

    #[tokio::test(start_paused = true)]
    async fn test_cache_cleanup_removes_expired_entries() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::from_millis(100));

        cache.insert("expired".to_string(), "old".to_string()).await;
        // Insert a second entry with a longer TTL
        cache
            .insert_with_ttl(
                "fresh".to_string(),
                "new".to_string(),
                Duration::from_secs(60),
            )
            .await;

        // Advance past the short TTL only
        tokio::time::advance(Duration::from_millis(150)).await;

        assert_eq!(cache.size().await, 2); // Both entries still present before cleanup

        cache.cleanup_expired().await;

        assert_eq!(
            cache.size().await,
            1,
            "expired entry should have been removed"
        );
        assert_eq!(
            cache.get(&"fresh".to_string()).await,
            Some("new".to_string()),
            "non-expired entry should still be present"
        );
        assert_eq!(
            cache.get(&"expired".to_string()).await,
            None,
            "expired entry should not be retrievable"
        );
    }

    #[tokio::test]
    async fn test_cache_overwrite_existing_key() {
        let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::from_secs(60));
        cache.insert("k".to_string(), 1u32).await;
        cache.insert("k".to_string(), 2u32).await;
        assert_eq!(cache.get(&"k".to_string()).await, Some(2u32));
    }

    #[tokio::test]
    async fn test_cache_remove() {
        let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::from_secs(60));
        cache.insert("k".to_string(), "v".to_string()).await;
        let removed = cache.remove(&"k".to_string()).await;
        assert_eq!(removed, Some("v".to_string()));
        assert_eq!(cache.get(&"k".to_string()).await, None);
    }
}
