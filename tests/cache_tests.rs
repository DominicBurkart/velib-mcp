use chrono::Duration;
use std::sync::Arc;
use std::time;
use velib_mcp::data::cache::InMemoryCache;

#[tokio::test]
async fn test_cache_insert_and_get() {
    let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::seconds(60));

    cache.insert("key1".to_string(), "value1".to_string()).await;

    let result = cache.get(&"key1".to_string()).await;
    assert_eq!(result, Some("value1".to_string()));

    // Non-existent key returns None
    let missing = cache.get(&"nonexistent".to_string()).await;
    assert_eq!(missing, None);
}

#[tokio::test]
async fn test_cache_default_ttl_expiry() {
    // Exercises the `insert` path (default TTL) rather than `insert_with_ttl`.
    // Cache TTL is driven by chrono::Utc::now(), so we cannot use
    // `tokio::time::pause()`; use a generous margin to avoid CI flakiness.
    let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::milliseconds(200));

    cache.insert("k".to_string(), "v".to_string()).await;

    // Present immediately.
    assert_eq!(cache.get(&"k".to_string()).await, Some("v".to_string()));

    // Wait well past the default TTL (4x margin) to absorb scheduling jitter.
    tokio::time::sleep(time::Duration::from_millis(800)).await;

    assert_eq!(cache.get(&"k".to_string()).await, None);
}

#[tokio::test]
async fn test_cache_expired_entry_returns_none() {
    let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::seconds(60));

    // Insert with a short TTL. The cache uses chrono::Utc::now() internally,
    // so tokio's virtual time helpers are not applicable; pick a TTL/sleep
    // pair with a large absolute margin so that scheduling jitter on shared
    // CI runners cannot cause flakes.
    cache
        .insert_with_ttl(
            "ephemeral".to_string(),
            "gone_soon".to_string(),
            Duration::milliseconds(200),
        )
        .await;

    // Should be present immediately
    assert_eq!(
        cache.get(&"ephemeral".to_string()).await,
        Some("gone_soon".to_string())
    );

    // Wait well past the TTL (4x margin) to absorb scheduling jitter.
    tokio::time::sleep(time::Duration::from_millis(800)).await;

    // Should now return None
    assert_eq!(cache.get(&"ephemeral".to_string()).await, None);
}

#[tokio::test]
async fn test_cleanup_removes_only_expired() {
    let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));

    // Insert one entry with a short TTL and one with a long TTL. As above we
    // use a wide margin (200ms TTL / 800ms sleep) instead of tokio's virtual
    // time because the cache reads from chrono::Utc::now().
    cache
        .insert_with_ttl("short".to_string(), 1, Duration::milliseconds(200))
        .await;
    cache
        .insert_with_ttl("long".to_string(), 2, Duration::seconds(60))
        .await;

    assert_eq!(cache.size().await, 2);

    // Wait for the short-lived entry to expire.
    tokio::time::sleep(time::Duration::from_millis(800)).await;

    cache.cleanup_expired().await;

    // Only the long-lived entry should remain
    assert_eq!(cache.size().await, 1);
    assert_eq!(cache.get(&"short".to_string()).await, None);
    assert_eq!(cache.get(&"long".to_string()).await, Some(2));
}

#[tokio::test]
async fn test_cache_overwrite() {
    let cache: InMemoryCache<String, String> = InMemoryCache::new(Duration::seconds(60));

    cache
        .insert("key".to_string(), "original".to_string())
        .await;
    assert_eq!(
        cache.get(&"key".to_string()).await,
        Some("original".to_string())
    );

    // Overwrite with a new value
    cache.insert("key".to_string(), "updated".to_string()).await;
    assert_eq!(
        cache.get(&"key".to_string()).await,
        Some("updated".to_string())
    );

    // Size should still be 1
    assert_eq!(cache.size().await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cache_concurrent_access() {
    // Exercises concurrent `insert` and `get` on the async-locked cache to
    // guard against deadlocks or data races. Each task writes its own key
    // then reads it back; we then verify all keys are present.
    let cache: Arc<InMemoryCache<String, u32>> =
        Arc::new(InMemoryCache::new(Duration::seconds(60)));

    let mut handles = Vec::new();
    for i in 0..32u32 {
        let cache = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            let key = format!("key-{i}");
            cache.insert(key.clone(), i).await;
            // Interleave reads of our own key and (possibly) others.
            let own = cache.get(&key).await;
            assert_eq!(own, Some(i));
        }));
    }

    for h in handles {
        h.await.expect("task panicked");
    }

    assert_eq!(cache.size().await, 32);
    for i in 0..32u32 {
        assert_eq!(cache.get(&format!("key-{i}")).await, Some(i));
    }
}

/// Integration-level cache test: concurrent reads and writes (including
/// cleanup_expired running in parallel) should not panic or deadlock.
#[tokio::test]
async fn cache_concurrent_access_does_not_deadlock() {
    let cache: Arc<InMemoryCache<u64, String>> =
        Arc::new(InMemoryCache::new(Duration::seconds(60)));

    let mut handles = Vec::new();

    // Spawn 20 concurrent writers
    for i in 0..20u64 {
        let cache = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            cache.insert(i, format!("value-{i}")).await;
        }));
    }

    // Spawn 20 concurrent readers
    for i in 0..20u64 {
        let cache = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            let _ = cache.get(&i).await;
        }));
    }

    // Spawn cleanup in parallel
    {
        let cache = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            cache.cleanup_expired().await;
        }));
    }

    for handle in handles {
        handle.await.expect("Task should not panic");
    }

    // All 20 writes should be visible
    assert_eq!(cache.size().await, 20);
}
