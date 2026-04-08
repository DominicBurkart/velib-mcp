//! Unit tests for `InMemoryCache` that run entirely offline.
//!
//! Key invariants validated:
//! - Fresh entries are returned; expired entries are not.
//! - `insert_with_ttl` overrides the default TTL.
//! - `remove` returns the value and shrinks the cache.
//! - `clear` empties the cache without error.
//! - `cleanup_expired` only evicts entries whose TTL has passed.

use chrono::Duration;
use velib_mcp::data::cache::InMemoryCache;

#[tokio::test]
async fn fresh_entry_is_returned() {
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("k".to_string(), 42).await;
    assert_eq!(cache.get(&"k".to_string()).await, Some(42));
}

#[tokio::test]
async fn expired_entry_is_not_returned() {
    // TTL of -1 minute means the entry is already expired on insertion.
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(-1));
    cache.insert("k".to_string(), 99).await;
    assert_eq!(cache.get(&"k".to_string()).await, None);
}

#[tokio::test]
async fn insert_with_ttl_overrides_default() {
    // Default TTL would be expired, but we override with a long TTL.
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(-1));
    cache
        .insert_with_ttl("k".to_string(), 7, Duration::minutes(10))
        .await;
    assert_eq!(cache.get(&"k".to_string()).await, Some(7));
}

#[tokio::test]
async fn remove_returns_value_and_shrinks_cache() {
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("a".to_string(), 1).await;
    cache.insert("b".to_string(), 2).await;

    let removed = cache.remove(&"a".to_string()).await;
    assert_eq!(removed, Some(1));
    assert_eq!(cache.size().await, 1);
    assert_eq!(cache.get(&"a".to_string()).await, None);
}

#[tokio::test]
async fn remove_missing_key_returns_none() {
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(5));
    assert_eq!(cache.remove(&"ghost".to_string()).await, None);
}

#[tokio::test]
async fn clear_empties_cache() {
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(5));
    cache.insert("x".to_string(), 1).await;
    cache.insert("y".to_string(), 2).await;
    assert_eq!(cache.size().await, 2);

    cache.clear().await;
    assert_eq!(cache.size().await, 0);
}

#[tokio::test]
async fn cleanup_expired_evicts_only_stale_entries() {
    let cache: InMemoryCache<String, u32> = InMemoryCache::new(Duration::minutes(5));
    // Insert one fresh entry and one pre-expired entry.
    cache.insert("fresh".to_string(), 1).await;
    cache
        .insert_with_ttl("stale".to_string(), 2, Duration::minutes(-1))
        .await;
    assert_eq!(cache.size().await, 2);

    cache.cleanup_expired().await;
    assert_eq!(cache.size().await, 1);
    assert_eq!(cache.get(&"fresh".to_string()).await, Some(1));
    assert_eq!(cache.get(&"stale".to_string()).await, None);
}

#[tokio::test]
async fn size_reflects_insertion_count() {
    let cache: InMemoryCache<u32, &str> = InMemoryCache::new(Duration::minutes(5));
    assert_eq!(cache.size().await, 0);
    cache.insert(1, "a").await;
    assert_eq!(cache.size().await, 1);
    cache.insert(2, "b").await;
    assert_eq!(cache.size().await, 2);
    // Re-inserting the same key does not grow the cache.
    cache.insert(1, "c").await;
    assert_eq!(cache.size().await, 2);
}
