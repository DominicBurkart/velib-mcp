use chrono::Duration;
use velib_mcp::data::cache::InMemoryCache;

/// Integration-level cache test: concurrent reads and writes should not panic or deadlock.
#[tokio::test]
async fn cache_concurrent_access_does_not_deadlock() {
    use std::sync::Arc;

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
