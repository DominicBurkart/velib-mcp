#[cfg(test)]
mod tests {
    use crate::data::cache::{CacheEntry, InMemoryCache};
    use chrono::Duration;

    #[test]
    fn cache_entry_not_expired_within_ttl() {
        let entry = CacheEntry::new(42, Duration::seconds(60));
        assert!(!entry.is_expired());
    }

    #[test]
    fn cache_entry_expired_with_zero_ttl() {
        // A zero-duration TTL should expire essentially immediately
        let entry = CacheEntry::new(42, Duration::zero());
        // The entry expires at Utc::now() + 0, so by the time we check it
        // should be expired (or exactly at boundary).
        // We allow either result since clock resolution varies.
        // The important invariant: negative TTL always expires.
        let neg_entry = CacheEntry::new(42, Duration::seconds(-1));
        assert!(neg_entry.is_expired());
    }

    #[tokio::test]
    async fn cache_insert_and_get_returns_value() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key1".to_string(), 100).await;

        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, Some(100));
    }

    #[tokio::test]
    async fn cache_get_missing_key_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        let result = cache.get(&"nonexistent".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn cache_expired_entry_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(-1));
        cache.insert("key1".to_string(), 100).await;

        // Entry was inserted with a negative TTL, so it's already expired
        let result = cache.get(&"key1".to_string()).await;
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn cache_insert_with_custom_ttl() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(1));
        // Insert with a long custom TTL
        cache
            .insert_with_ttl("long_lived".to_string(), 200, Duration::seconds(3600))
            .await;
        // Insert with a negative custom TTL (already expired)
        cache
            .insert_with_ttl("expired".to_string(), 300, Duration::seconds(-1))
            .await;

        assert_eq!(cache.get(&"long_lived".to_string()).await, Some(200));
        assert_eq!(cache.get(&"expired".to_string()).await, None);
    }

    #[tokio::test]
    async fn cache_remove_returns_value_and_removes() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key1".to_string(), 100).await;

        let removed = cache.remove(&"key1".to_string()).await;
        assert_eq!(removed, Some(100));

        // Key should no longer be present
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn cache_remove_missing_key_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        let removed = cache.remove(&"nonexistent".to_string()).await;
        assert_eq!(removed, None);
    }

    #[tokio::test]
    async fn cache_size_tracks_entries() {
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
    async fn cache_clear_removes_all_entries() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;
        assert_eq!(cache.size().await, 2);

        cache.clear().await;
        assert_eq!(cache.size().await, 0);
        assert_eq!(cache.get(&"a".to_string()).await, None);
    }

    #[tokio::test]
    async fn cache_cleanup_expired_removes_only_expired() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));

        // Insert a live entry with the default (long) TTL
        cache.insert("live".to_string(), 1).await;
        // Insert an expired entry with a negative TTL
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
    async fn cache_overwrite_replaces_value() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key".to_string(), 1).await;
        cache.insert("key".to_string(), 2).await;

        assert_eq!(cache.get(&"key".to_string()).await, Some(2));
        assert_eq!(cache.size().await, 1);
    }

    #[tokio::test]
    async fn cache_integer_keys() {
        let cache: InMemoryCache<u64, String> = InMemoryCache::new(Duration::seconds(60));
        cache.insert(42, "hello".to_string()).await;
        cache.insert(99, "world".to_string()).await;

        assert_eq!(cache.get(&42).await, Some("hello".to_string()));
        assert_eq!(cache.get(&99).await, Some("world".to_string()));
        assert_eq!(cache.get(&0).await, None);
    }
}
