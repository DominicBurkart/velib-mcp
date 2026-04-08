//! Unit tests for InMemoryCache.
//!
//! All tests are offline — no network access required.

use chrono::Duration;
use velib_mcp::data::cache::InMemoryCache;

// ── basic get/insert ────────────────────────────────────────────────────────

#[tokio::test]
async fn cache_miss_on_empty() {
    let cache: InMemoryCache<&str, u32> = InMemoryCache::new(Duration::minutes(5));
    assert!(cache.get(&"missing").await.is_none());
}

#[tokio::test]
async fn cache_hit_after_insert() {
    let cache = InMemoryCache::new(Duration::minutes(5));
    cache.insert("key", 42u32).await;
    assert_eq!(cache.get(&"key").await, Some(42));
}

#[tokio::test]
async fn cache_miss_after_remove() {
    let cache = InMemoryCache::new(Duration::minutes(5));
    cache.insert("key", 42u32).await;
    let removed = cache.remove(&"key").await;
    assert_eq!(removed, Some(42));
    assert!(cache.get(&"key").await.is_none());
}

#[tokio::test]
async fn cache_size_tracks_insertions() {
    let cache = InMemoryCache::new(Duration::minutes(5));
    assert_eq!(cache.size().await, 0);
    cache.insert("a", 1u32).await;
    cache.insert("b", 2u32).await;
    assert_eq!(cache.size().await, 2);
}

#[tokio::test]
async fn cache_clear_empties_all_entries() {
    let cache = InMemoryCache::new(Duration::minutes(5));
    cache.insert("a", 1u32).await;
    cache.insert("b", 2u32).await;
    cache.clear().await;
    assert_eq!(cache.size().await, 0);
}

// ── TTL / expiry ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn expired_entry_returns_none() {
    // Insert with a negative TTL so the entry is immediately expired.
    let cache: InMemoryCache<&str, u32> = InMemoryCache::new(Duration::seconds(-1));
    cache.insert("key", 99u32).await;
    // The entry exists in the map but its expires_at is in the past.
    assert!(cache.get(&"key").await.is_none());
}

#[tokio::test]
async fn insert_with_ttl_overrides_default() {
    // Default TTL is generous; per-entry TTL is already expired.
    let cache: InMemoryCache<&str, u32> = InMemoryCache::new(Duration::minutes(5));
    cache
        .insert_with_ttl("key", 7u32, Duration::seconds(-1))
        .await;
    assert!(cache.get(&"key").await.is_none());
}

#[tokio::test]
async fn cleanup_expired_removes_stale_keeps_fresh() {
    let cache: InMemoryCache<&str, u32> = InMemoryCache::new(Duration::minutes(5));

    // Stale entry — negative TTL.
    cache
        .insert_with_ttl("stale", 1u32, Duration::seconds(-1))
        .await;
    // Fresh entry — positive TTL.
    cache
        .insert_with_ttl("fresh", 2u32, Duration::minutes(5))
        .await;

    assert_eq!(cache.size().await, 2);
    cache.cleanup_expired().await;
    assert_eq!(cache.size().await, 1);
    assert_eq!(cache.get(&"fresh").await, Some(2));
}

// ── overwrite behaviour ───────────────────────────────────────────────────────

#[tokio::test]
async fn insert_overwrites_existing_key() {
    let cache = InMemoryCache::new(Duration::minutes(5));
    cache.insert("key", 1u32).await;
    cache.insert("key", 2u32).await;
    assert_eq!(cache.get(&"key").await, Some(2));
    assert_eq!(cache.size().await, 1);
}
