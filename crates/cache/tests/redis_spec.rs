#![cfg(feature = "redis")]

use std::{sync::Arc, time::Duration};

use cache::{CacheEntry, CacheStore, DEFAULT_REDIS_KEY_PREFIX, RedisCacheStore};
use redis::{RedisCommands, RedisFake};

#[tokio::test]
async fn test_redis_cache_store_crud() {
    let fake_redis = Arc::new(RedisFake::new());
    let store = RedisCacheStore::new(fake_redis.clone());

    assert_eq!(store.prefix(), DEFAULT_REDIS_KEY_PREFIX);
    assert_eq!(store.format_key("user:1"), "druto:cache:user:1");
    assert!(store.client().ping().await.is_ok());

    let entry = CacheEntry::from_value(
        &"alice",
        Duration::from_secs(3600),
        Duration::ZERO,
        Vec::new(),
    )
    .unwrap();
    store
        .put("user:1", &entry, Duration::from_secs(3600))
        .await
        .unwrap();

    let fetched = store
        .get("user:1")
        .await
        .unwrap()
        .expect("redis entry exists");
    assert_eq!(fetched.deserialize::<String>().unwrap(), "alice");

    // Add existing returns false.
    let added = store
        .add("user:1", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    assert!(!added);

    // Add new returns true.
    let added_new = store
        .add("user:2", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    assert!(added_new);

    // Pull.
    let pulled = store.pull("user:2").await.unwrap().expect("pulled entry");
    assert_eq!(pulled.deserialize::<String>().unwrap(), "alice");
    assert!(store.get("user:2").await.unwrap().is_none());

    // Increments and decrements.
    let count1 = store
        .increment("visits", 10, Duration::from_secs(3600))
        .await
        .unwrap();
    assert_eq!(count1, 10);
    let count2 = store
        .decrement("visits", 4, Duration::from_secs(3600))
        .await
        .unwrap();
    assert_eq!(count2, 6);

    // Forget and forget many.
    assert!(store.forget("user:1").await.unwrap());
    assert!(!store.forget("user:1").await.unwrap());

    store
        .put("k1", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    store
        .put("k2", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    let removed = store
        .forget_many(&["k1", "k2", "nonexistent"])
        .await
        .unwrap();
    assert_eq!(removed, 2);

    // Flush.
    store
        .put("k3", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    store.flush().await.unwrap();
    assert!(store.get("k3").await.unwrap().is_none());
}

#[tokio::test]
async fn test_redis_cache_store_custom_prefix() {
    let fake_redis = Arc::new(RedisFake::new());
    let store = RedisCacheStore::with_prefix(fake_redis.clone(), "custom:cache:");

    assert_eq!(store.prefix(), "custom:cache:");
    assert_eq!(store.format_key("key"), "custom:cache:key");

    let entry =
        CacheEntry::from_value(&123, Duration::from_secs(60), Duration::ZERO, Vec::new()).unwrap();
    store
        .put("key", &entry, Duration::from_secs(60))
        .await
        .unwrap();

    let fetched = store.get("key").await.unwrap().unwrap();
    assert_eq!(fetched.deserialize::<i32>().unwrap(), 123);
}
