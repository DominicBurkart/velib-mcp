use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
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
    entries: RwLock<HashMap<K, CacheEntry<V>>>,
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
            entries: RwLock::new(HashMap::new()),
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

    #[test]
    fn cache_entry_not_expired_within_ttl() {
        let entry = CacheEntry::new(42, Duration::seconds(60));
        assert!(!entry.is_expired());
    }

    #[test]
    fn cache_entry_expired_with_negative_ttl() {
        let entry = CacheEntry::new(42, Duration::seconds(-1));
        assert!(entry.is_expired());
    }

    #[tokio::test]
    async fn insert_and_get_returns_value() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key1".to_string(), 100).await;

        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some(100));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        let result = cache.get(&"nonexistent".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn expired_entry_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(-1));
        cache.insert("key1".to_string(), 100).await;

        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn insert_with_custom_ttl() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(1));
        cache
            .insert_with_ttl("long_lived".to_string(), 200, Duration::seconds(3600))
            .await;
        cache
            .insert_with_ttl("expired".to_string(), 300, Duration::seconds(-1))
            .await;

        assert_eq!(cache.get(&"long_lived".to_string()).await, Some(200));
        assert_eq!(cache.get(&"expired".to_string()).await, None);
    }

    #[tokio::test]
    async fn remove_returns_value_and_deletes() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key1".to_string(), 100).await;

        let removed = cache.remove(&"key1".to_string()).await;
        assert_eq!(removed, Some(100));
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn remove_missing_key_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        let removed = cache.remove(&"nonexistent".to_string()).await;
        assert_eq!(removed, None);
    }

    #[tokio::test]
    async fn size_tracks_entries() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        assert_eq!(cache.size().await, 0);

        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;
        cache.insert("c".to_string(), 3).await;
        assert_eq!(cache.size().await, 3);

        cache.remove(&"b".to_string()).await;
        assert_eq!(cache.size().await, 2);
    }

    #[tokio::test]
    async fn clear_removes_all_entries() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;
        assert_eq!(cache.size().await, 2);

        cache.clear().await;
        assert_eq!(cache.size().await, 0);
        assert_eq!(cache.get(&"a".to_string()).await, None);
    }

    #[tokio::test]
    async fn cleanup_expired_removes_only_expired() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));

        cache.insert("live".to_string(), 1).await;
        cache
            .insert_with_ttl("expired".to_string(), 2, Duration::seconds(-1))
            .await;

        assert_eq!(cache.size().await, 2);

        cache.cleanup_expired().await;

        assert_eq!(cache.size().await, 1);
        assert_eq!(cache.get(&"live".to_string()).await, Some(1));
        assert_eq!(cache.get(&"expired".to_string()).await, None);
    }

    #[tokio::test]
    async fn overwrite_replaces_value() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key".to_string(), 1).await;
        cache.insert("key".to_string(), 2).await;

        assert_eq!(cache.get(&"key".to_string()).await, Some(2));
        assert_eq!(cache.size().await, 1);
    }

    #[tokio::test]
    async fn integer_keys_work() {
        let cache: InMemoryCache<u64, String> = InMemoryCache::new(Duration::seconds(60));
        cache.insert(42, "hello".to_string()).await;
        cache.insert(99, "world".to_string()).await;

        assert_eq!(cache.get(&42).await, Some("hello".to_string()));
        assert_eq!(cache.get(&99).await, Some("world".to_string()));
        assert_eq!(cache.get(&0).await, None);
    }
}
