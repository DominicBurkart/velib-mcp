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

    /// Returns the raw `(hits, misses)` counters for this cache.
    ///
    /// Use these to compute a pooled hit rate across multiple caches rather
    /// than averaging per-cache rates, which is inaccurate when the caches
    /// receive different numbers of lookups.
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
    /// Note: when a key is present but its TTL has expired, the lookup is
    /// counted as a **miss** (the expired entry is not returned).  In a
    /// cache with a small number of keys this produces exactly one stale-key
    /// miss per TTL boundary before the entry is refreshed, which slightly
    /// depresses the reported rate.  If you need a rate unaffected by
    /// expiry events, use [`stats`] to obtain the raw counters and exclude
    /// the stale lookups tracked by a separate counter in your own code.
    ///
    /// The sum `hits + misses` is computed with saturating arithmetic so the
    /// result is well-defined even after a very large number of lookups in a
    /// long-running process.
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
    use std::time::Duration as StdDuration;

    /// Fresh cache — no lookups yet → hit_rate() must be 0.0 and stats (0,0).
    #[tokio::test]
    async fn hit_rate_fresh_cache() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::minutes(5));
        assert_eq!(cache.hit_rate(), 0.0);
        assert_eq!(cache.stats(), (0, 0));
    }

    /// insert + get hit → hit_rate() == 1.0.
    #[tokio::test]
    async fn hit_rate_one_hit() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::minutes(5));
        cache.insert("k".to_string(), "v".to_string()).await;
        let result = cache.get(&"k".to_string()).await;
        assert_eq!(result, Some("v".to_string()));
        assert_eq!(cache.hit_rate(), 1.0);
        assert_eq!(cache.stats(), (1, 0));
    }

    /// One hit + one miss on a different key → hit_rate() == 0.5.
    #[tokio::test]
    async fn hit_rate_one_hit_one_miss() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::minutes(5));
        cache.insert("present".to_string(), "v".to_string()).await;
        let _ = cache.get(&"present".to_string()).await; // hit
        let _ = cache.get(&"absent".to_string()).await;  // miss
        assert!((cache.hit_rate() - 0.5).abs() < f64::EPSILON);
        assert_eq!(cache.stats(), (1, 1));
    }

    /// Expired entry: lookup counts as a miss (not a hit).
    #[tokio::test]
    async fn expired_entry_counts_as_miss() {
        let cache: InMemoryCache<String, String> =
            InMemoryCache::new(Duration::milliseconds(1));
        cache.insert("k".to_string(), "v".to_string()).await;
        // Wait long enough for the 1 ms TTL to lapse.
        tokio::time::sleep(StdDuration::from_millis(20)).await;
        let result = cache.get(&"k".to_string()).await;
        // The key was found but expired → returned None and counted as miss.
        assert!(result.is_none());
        assert_eq!(cache.stats(), (0, 1));
    }
}
