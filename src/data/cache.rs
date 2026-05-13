use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
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
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.data.clone());
            }
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Returns the raw `(hits, misses)` counters accumulated since the cache
    /// was created. Use these to pool counts across multiple caches before
    /// computing a combined rate, or to inspect the raw numbers directly.
    ///
    /// An expired-key lookup (key present but TTL elapsed) increments
    /// `misses`, not `hits`.
    #[must_use]
    pub fn stats(&self) -> (u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }

    /// Returns the cache hit rate as a value in [0.0, 1.0].
    /// Returns 0.0 when no lookups have been performed yet.
    ///
    /// **Stale-entry behaviour**: when a key is present but its TTL has
    /// elapsed, `get` counts the lookup as a *miss*. In a single-key-per-cache
    /// setup (e.g. `"all_reference_stations"`), this produces exactly one
    /// stale-key miss per TTL boundary before the entry is refreshed, which
    /// slightly depresses the reported rate. Callers that need a rate
    /// unaffected by this should use the raw `stats()` counters and apply
    /// their own denominator policy.
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits.saturating_add(misses);
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
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

    // --- hit_rate and stats ---

    #[tokio::test]
    async fn hit_rate_fresh_cache() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        assert_eq!(cache.hit_rate(), 0.0);
        assert_eq!(cache.stats(), (0, 0));
    }

    #[tokio::test]
    async fn hit_rate_one_hit() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("k".to_string(), 1).await;
        cache.get(&"k".to_string()).await;
        assert_eq!(cache.hit_rate(), 1.0);
        assert_eq!(cache.stats(), (1, 0));
    }

    #[tokio::test]
    async fn hit_rate_one_hit_one_miss() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("k".to_string(), 1).await;
        cache.get(&"k".to_string()).await; // hit
        cache.get(&"missing".to_string()).await; // miss
        assert!((cache.hit_rate() - 0.5).abs() < 1e-9);
        assert_eq!(cache.stats(), (1, 1));
    }

    #[tokio::test]
    async fn hit_rate_zero_when_no_lookups() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[tokio::test]
    async fn hit_rate_one_after_only_hits() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("k".to_string(), 1).await;
        cache.get(&"k".to_string()).await;
        cache.get(&"k".to_string()).await;
        assert_eq!(cache.hit_rate(), 1.0);
    }

    #[tokio::test]
    async fn hit_rate_zero_after_only_misses() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.get(&"missing".to_string()).await;
        assert_eq!(cache.hit_rate(), 0.0);
    }

    #[tokio::test]
    async fn hit_rate_half_after_equal_hits_and_misses() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("k".to_string(), 1).await;
        cache.get(&"k".to_string()).await; // hit
        cache.get(&"missing".to_string()).await; // miss
        assert!((cache.hit_rate() - 0.5).abs() < 1e-9);
    }

    /// An expired-key lookup (key present but TTL elapsed) must be counted as a
    /// miss, not a hit. This locks in the documented behaviour so future
    /// refactors cannot silently change the semantics.
    #[tokio::test]
    async fn expired_entry_counts_as_miss() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        // Insert with a TTL that is already in the past.
        cache
            .insert_with_ttl("k".to_string(), 1, Duration::milliseconds(-1))
            .await;
        let result = cache.get(&"k".to_string()).await;
        assert_eq!(result, None);
        // The stale lookup must be recorded as a miss, not a hit.
        assert_eq!(cache.stats(), (0, 1));
        assert_eq!(cache.hit_rate(), 0.0);
    }
}
