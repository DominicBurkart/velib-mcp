#[cfg(test)]
mod tests {
    use crate::data::cache::{CacheEntry, InMemoryCache};
    use chrono::Duration;

    #[test]
    fn cache_entry_not_expired_when_fresh() {
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
        cache.insert("key".into(), 99).await;
        assert_eq!(cache.get(&"key".into()).await, Some(99));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        assert_eq!(cache.get(&"missing".into()).await, None);
    }

    #[tokio::test]
    async fn expired_entry_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(-1));
        cache.insert("key".into(), 1).await;
        assert_eq!(cache.get(&"key".into()).await, None);
    }

    #[tokio::test]
    async fn insert_with_custom_ttl() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(-1));
        // Default TTL is expired, but custom TTL is long
        cache
            .insert_with_ttl("key".into(), 7, Duration::seconds(300))
            .await;
        assert_eq!(cache.get(&"key".into()).await, Some(7));
    }

    #[tokio::test]
    async fn remove_returns_value_and_deletes() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key".into(), 5).await;
        assert_eq!(cache.remove(&"key".into()).await, Some(5));
        assert_eq!(cache.get(&"key".into()).await, None);
    }

    #[tokio::test]
    async fn remove_missing_returns_none() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        assert_eq!(cache.remove(&"nope".into()).await, None);
    }

    #[tokio::test]
    async fn cleanup_removes_expired_entries() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        // Insert one with long TTL, one already expired
        cache.insert("alive".into(), 1).await;
        cache
            .insert_with_ttl("dead".into(), 2, Duration::seconds(-1))
            .await;
        assert_eq!(cache.size().await, 2);

        cache.cleanup_expired().await;
        assert_eq!(cache.size().await, 1);
        assert_eq!(cache.get(&"alive".into()).await, Some(1));
    }

    #[tokio::test]
    async fn clear_empties_cache() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("a".into(), 1).await;
        cache.insert("b".into(), 2).await;
        assert_eq!(cache.size().await, 2);

        cache.clear().await;
        assert_eq!(cache.size().await, 0);
    }

    #[tokio::test]
    async fn overwrite_key_updates_value() {
        let cache: InMemoryCache<String, i32> = InMemoryCache::new(Duration::seconds(60));
        cache.insert("key".into(), 1).await;
        cache.insert("key".into(), 2).await;
        assert_eq!(cache.get(&"key".into()).await, Some(2));
        assert_eq!(cache.size().await, 1);
    }
}
