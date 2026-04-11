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

    // ── helpers ──────────────────────────────────────────────────────────────

    fn long_ttl() -> Duration {
        Duration::minutes(10)
    }

    fn expired_ttl() -> Duration {
        // Negative TTL → entry is already past its expiry the moment it is
        // inserted, so is_expired() returns true immediately.
        Duration::seconds(-1)
    }

    // ── CacheEntry ───────────────────────────────────────────────────────────

    #[test]
    fn fresh_entry_is_not_expired() {
        let entry = CacheEntry::new(42_u32, long_ttl());
        assert!(!entry.is_expired());
    }

    #[test]
    fn expired_entry_reports_expired() {
        let entry = CacheEntry::new(42_u32, expired_ttl());
        assert!(entry.is_expired());
    }

    // ── InMemoryCache – basic insert / get ───────────────────────────────────

    #[tokio::test]
    async fn empty_cache_returns_none() {
        let cache: InMemoryCache<&str, u32> = InMemoryCache::new(long_ttl());
        assert!(cache.get(&"key").await.is_none());
    }

    #[tokio::test]
    async fn inserted_value_is_retrievable() {
        let cache = InMemoryCache::new(long_ttl());
        cache.insert("station:001", "Châtelet").await;
        assert_eq!(cache.get(&"station:001").await, Some("Châtelet"));
    }

    #[tokio::test]
    async fn overwrite_replaces_value() {
        let cache = InMemoryCache::new(long_ttl());
        cache.insert("k", 1_u32).await;
        cache.insert("k", 2_u32).await;
        assert_eq!(cache.get(&"k").await, Some(2));
    }

    // ── TTL / expiry ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn expired_entry_returns_none_on_get() {
        // Insert with a TTL that has already elapsed.
        let cache: InMemoryCache<&str, u32> = InMemoryCache::new(expired_ttl());
        cache.insert("k", 99).await;
        // The entry sits in the map but is_expired() is true → get returns None.
        assert!(cache.get(&"k").await.is_none());
    }

    // ── size ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn size_counts_all_entries_including_expired() {
        // size() counts physical map entries, not just live ones.  This is
        // intentional: cleanup_expired is a separate operation.
        let cache: InMemoryCache<&str, u32> = InMemoryCache::new(expired_ttl());
        cache.insert("a", 1).await;
        cache.insert("b", 2).await;
        assert_eq!(cache.size().await, 2);
    }

    #[tokio::test]
    async fn size_increases_on_insert() {
        let cache = InMemoryCache::new(long_ttl());
        assert_eq!(cache.size().await, 0);
        cache.insert("x", 10_u32).await;
        assert_eq!(cache.size().await, 1);
        cache.insert("y", 20_u32).await;
        assert_eq!(cache.size().await, 2);
    }

    // ── cleanup_expired ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn cleanup_removes_expired_entries() {
        let cache: InMemoryCache<&str, u32> = InMemoryCache::new(long_ttl());

        // Insert one live entry (long TTL) and one already-expired entry by
        // manually backdating its expires_at through the public insert.
        // We achieve the expired entry by using a cache with negative TTL.
        let expired_cache: InMemoryCache<&str, u32> = InMemoryCache::new(expired_ttl());
        expired_cache.insert("dead", 0).await;

        // For the "dead" entry we recreate the scenario within a single cache
        // that can hold both, by reaching through two separate caches and
        // asserting behavior instead. The key contract: after cleanup, expired
        // entries are gone.
        expired_cache.cleanup_expired().await;
        assert_eq!(expired_cache.size().await, 0);
    }

    #[tokio::test]
    async fn cleanup_preserves_live_entries() {
        let cache = InMemoryCache::new(long_ttl());
        cache.insert("live", 42_u32).await;
        cache.cleanup_expired().await;
        assert_eq!(cache.size().await, 1);
        assert_eq!(cache.get(&"live").await, Some(42));
    }

    #[tokio::test]
    async fn cleanup_on_empty_cache_is_harmless() {
        let cache: InMemoryCache<&str, u32> = InMemoryCache::new(long_ttl());
        cache.cleanup_expired().await; // Must not panic
        assert_eq!(cache.size().await, 0);
    }

    // ── clear ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn clear_removes_all_entries() {
        let cache = InMemoryCache::new(long_ttl());
        cache.insert("a", 1_u32).await;
        cache.insert("b", 2_u32).await;
        cache.insert("c", 3_u32).await;
        cache.clear().await;
        assert_eq!(cache.size().await, 0);
        assert!(cache.get(&"a").await.is_none());
    }

    #[tokio::test]
    async fn clear_on_empty_cache_is_harmless() {
        let cache: InMemoryCache<&str, u32> = InMemoryCache::new(long_ttl());
        cache.clear().await; // Must not panic
        assert_eq!(cache.size().await, 0);
    }

    // ── distinct keys ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn distinct_keys_do_not_interfere() {
        let cache = InMemoryCache::new(long_ttl());
        cache.insert("alpha", 1_u32).await;
        cache.insert("beta", 2_u32).await;
        assert_eq!(cache.get(&"alpha").await, Some(1));
        assert_eq!(cache.get(&"beta").await, Some(2));
        assert!(cache.get(&"gamma").await.is_none());
    }
}
