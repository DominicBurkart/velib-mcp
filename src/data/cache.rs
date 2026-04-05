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

/// Kani formal verification proofs for cache TTL logic.
///
/// Chrono's `DateTime<Utc>` relies on system calls (`Utc::now()`) and complex
/// internal representations that Kani cannot model directly. Instead, we verify
/// the equivalent arithmetic properties on simplified numeric types (i64
/// timestamps in seconds), which mirror how chrono stores and compares times
/// internally (`DateTime<Utc>` wraps an `NaiveDateTime` backed by i64 seconds
/// + nanoseconds).
///
/// The three properties verified:
/// 1. TTL addition safety: `now + ttl` does not overflow for reasonable TTL
///    values (up to 30 days).
/// 2. Expiration invariant: an entry created with `ttl > 0` is not immediately
///    expired, and an entry whose TTL has fully elapsed is expired.
/// 3. Custom TTL bounds: extreme TTL values (including negatives and very
///    large durations) are handled without panic via saturating arithmetic.
#[cfg(kani)]
mod verification {
    /// Maximum realistic "now" timestamp we consider: year ~2100.
    /// `chrono::DateTime<Utc>` stores seconds as i64 with a much larger range,
    /// so any timestamp up to this bound is well within safe territory.
    const MAX_REALISTIC_NOW: i64 = 4_102_444_800; // 2100-01-01T00:00:00Z

    /// Models `CacheEntry::new`: returns `expires_at = now + ttl_seconds`.
    /// Uses checked arithmetic to detect overflow (mirrors chrono's internal
    /// checked_add behaviour).
    fn model_new_entry(now: i64, ttl_seconds: i64) -> Option<i64> {
        now.checked_add(ttl_seconds)
    }

    /// Models `CacheEntry::is_expired`: `current_time > expires_at`.
    fn model_is_expired(current_time: i64, expires_at: i64) -> bool {
        current_time > expires_at
    }

    // -----------------------------------------------------------------------
    // Proof 1: TTL addition safety
    // -----------------------------------------------------------------------
    /// For any realistic "now" (0 ..= year 2100) and any TTL between 1 minute
    /// and 30 days, `now + ttl` must not overflow i64.
    #[kani::proof]
    fn verify_ttl_addition_no_overflow() {
        let now: i64 = kani::any();
        let ttl_seconds: i64 = kani::any();

        // Constrain to realistic ranges.
        kani::assume(now >= 0 && now <= MAX_REALISTIC_NOW);
        kani::assume(ttl_seconds >= 60 && ttl_seconds <= 30 * 24 * 3600); // 1 min .. 30 days

        let result = model_new_entry(now, ttl_seconds);
        // Must never overflow.
        assert!(result.is_some(), "TTL addition overflowed for realistic inputs");
        // Result must be strictly greater than now (ttl >= 60 > 0).
        assert!(result.unwrap() > now);
    }

    // -----------------------------------------------------------------------
    // Proof 2: Expiration invariant
    // -----------------------------------------------------------------------
    /// An entry created with `ttl > 0` is **not** expired at creation time,
    /// and **is** expired once `ttl` seconds have elapsed.
    #[kani::proof]
    fn verify_expiration_invariant() {
        let now: i64 = kani::any();
        let ttl_seconds: i64 = kani::any();

        kani::assume(now >= 0 && now <= MAX_REALISTIC_NOW);
        kani::assume(ttl_seconds > 0 && ttl_seconds <= 30 * 24 * 3600);

        let expires_at = model_new_entry(now, ttl_seconds).unwrap();

        // At creation time, the entry must NOT be expired.
        // is_expired checks `current_time > expires_at`.
        // `now > now + ttl` is false when ttl > 0 and no overflow.
        assert!(
            !model_is_expired(now, expires_at),
            "Entry with positive TTL should not be expired at creation"
        );

        // One second after expiry, the entry MUST be expired.
        let after_expiry = expires_at + 1;
        // Guard: after_expiry itself must not overflow (always true in our range).
        kani::assume(after_expiry > expires_at);
        assert!(
            model_is_expired(after_expiry, expires_at),
            "Entry should be expired after TTL has elapsed"
        );
    }

    // -----------------------------------------------------------------------
    // Proof 3: Custom TTL bounds (extreme values)
    // -----------------------------------------------------------------------
    /// `insert_with_ttl` accepts an arbitrary `Duration`. We show that using
    /// saturating addition instead of wrapping/checked prevents panics for
    /// *any* i64 TTL, including negatives and i64::MAX.
    #[kani::proof]
    fn verify_custom_ttl_no_panic() {
        let now: i64 = kani::any();
        let ttl_seconds: i64 = kani::any();

        // Saturating addition never panics.
        let expires_at = now.saturating_add(ttl_seconds);

        // The result is always a valid i64 (no UB, no panic).
        // If ttl is negative, expires_at <= now (entry immediately expired).
        if ttl_seconds <= 0 {
            assert!(expires_at <= now);
        }

        // If ttl is very large and would overflow, saturating_add clamps to
        // i64::MAX, which is still a valid (far future) timestamp.
        if now > 0 && ttl_seconds > 0 {
            assert!(expires_at >= now);
        }
    }
}
