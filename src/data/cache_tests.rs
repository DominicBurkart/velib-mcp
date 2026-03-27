use super::cache::{CacheEntry, InMemoryCache};
use chrono::Duration;

#[test]
fn cache_entry_not_expired_within_ttl() {
    let entry = CacheEntry::new(42, Duration::seconds(60));
    assert!(!entry.is_expired());
}

#[test]
fn cache_entry_expired_with_negative_ttl() {
    // A TTL in the past should be immediately expired
    let entry = CacheEntry::new(42, Duration::seconds(-1));
    assert!(entry.is_expired());
}

#[test]
fn cache_entry_expired_with_zero_ttl() {
    // Zero TTL means expires at the instant of creation;
    // by the time we check, it should be expired (or borderline).
    let entry = CacheEntry::new("hello", Duration::seconds(0));
    assert!(entry.is_expired());
}

#[tokio::test]
async fn insert_then_get_returns_value() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
    cache.insert("key".into(), 100).await;

    let val = cache.get(&"key".into()).await;
    assert_eq!(val, Some(100));
}

#[tokio::test]
async fn get_missing_key_returns_none() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
    assert_eq!(cache.get(&"missing".into()).await, None);
}

#[tokio::test]
async fn expired_entry_returns_none() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(-1));
    cache.insert("key".into(), 99).await;

    // Entry was inserted with a TTL that has already passed
    assert_eq!(cache.get(&"key".into()).await, None);
}

#[tokio::test]
async fn insert_with_custom_ttl_overrides_default() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
    // Insert with a TTL that is already expired
    cache
        .insert_with_ttl("short".into(), 1, Duration::seconds(-1))
        .await;
    // Insert with a long TTL
    cache
        .insert_with_ttl("long".into(), 2, Duration::seconds(300))
        .await;

    assert_eq!(cache.get(&"short".into()).await, None);
    assert_eq!(cache.get(&"long".into()).await, Some(2));
}

#[tokio::test]
async fn remove_returns_value_and_evicts() {
    let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::seconds(60));
    cache.insert("k".into(), "v".into()).await;

    let removed = cache.remove(&"k".into()).await;
    assert_eq!(removed, Some("v".into()));

    // Subsequent get should return None
    assert_eq!(cache.get(&"k".into()).await, None);
}

#[tokio::test]
async fn remove_missing_key_returns_none() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
    assert_eq!(cache.remove(&"nope".into()).await, None);
}

#[tokio::test]
async fn size_tracks_insertions() {
    let cache: InMemoryCache<u32, u32> = InMemoryCache::new(Duration::seconds(60));
    assert_eq!(cache.size().await, 0);

    cache.insert(1, 10).await;
    cache.insert(2, 20).await;
    assert_eq!(cache.size().await, 2);

    // Overwriting a key should not increase the count
    cache.insert(1, 11).await;
    assert_eq!(cache.size().await, 2);
}

#[tokio::test]
async fn clear_empties_cache() {
    let cache: InMemoryCache<u32, u32> = InMemoryCache::new(Duration::seconds(60));
    cache.insert(1, 10).await;
    cache.insert(2, 20).await;
    assert_eq!(cache.size().await, 2);

    cache.clear().await;
    assert_eq!(cache.size().await, 0);
    assert_eq!(cache.get(&1).await, None);
}

#[tokio::test]
async fn cleanup_expired_removes_only_stale_entries() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));

    // Insert a fresh entry with long TTL
    cache
        .insert_with_ttl("fresh".into(), 1, Duration::seconds(300))
        .await;
    // Insert a stale entry with negative TTL
    cache
        .insert_with_ttl("stale".into(), 2, Duration::seconds(-1))
        .await;

    assert_eq!(cache.size().await, 2);

    cache.cleanup_expired().await;

    assert_eq!(cache.size().await, 1);
    assert_eq!(cache.get(&"fresh".into()).await, Some(1));
    assert_eq!(cache.get(&"stale".into()).await, None);
}

#[tokio::test]
async fn overwrite_refreshes_ttl() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));

    // First insert with short (expired) TTL
    cache
        .insert_with_ttl("k".into(), 1, Duration::seconds(-1))
        .await;
    assert_eq!(cache.get(&"k".into()).await, None);

    // Overwrite with long TTL
    cache
        .insert_with_ttl("k".into(), 2, Duration::seconds(300))
        .await;
    assert_eq!(cache.get(&"k".into()).await, Some(2));
}
