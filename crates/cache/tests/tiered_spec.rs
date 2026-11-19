#![cfg(feature = "redis")]

use std::{sync::Arc, time::Duration};

use cache::{CacheEntry, CacheStore, MemoryCacheStore, RedisCacheStore, TieredCacheStore};
use redis::RedisFake;

#[tokio::test]
async fn test_tiered_cache_store_flow_and_backfill() {
    let memory = MemoryCacheStore::new();
    let fake_redis = Arc::new(RedisFake::new());
    let redis_store = Arc::new(RedisCacheStore::new(fake_redis));

    let tiered = TieredCacheStore::new(memory.clone(), redis_store.clone());

    let entry = CacheEntry::from_value(
        &"tiered_value",
        Duration::from_secs(3600),
        Duration::ZERO,
        Vec::new(),
    )
    .unwrap();

    // 1. Put writes to both L1 memory and L2 Redis.
    tiered
        .put("item1", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    assert!(memory.get("item1").await.unwrap().is_some());
    assert!(redis_store.get("item1").await.unwrap().is_some());

    // 2. Clear L1 memory; verify L1 miss fetches from L2 and backfills L1.
    memory.flush().await.unwrap();
    assert!(memory.get("item1").await.unwrap().is_none());

    let fetched = tiered.get("item1").await.unwrap().expect("fetched from L2");
    assert_eq!(fetched.deserialize::<String>().unwrap(), "tiered_value");

    // L1 must now be backfilled!
    assert!(memory.get("item1").await.unwrap().is_some());

    // 3. Add operation.
    assert!(
        !tiered
            .add("item1", &entry, Duration::from_secs(3600))
            .await
            .unwrap()
    );
    assert!(
        tiered
            .add("item2", &entry, Duration::from_secs(3600))
            .await
            .unwrap()
    );

    // 4. Forget removes from both L1 and L2.
    assert!(tiered.forget("item1").await.unwrap());
    assert!(memory.get("item1").await.unwrap().is_none());
    assert!(redis_store.get("item1").await.unwrap().is_none());

    // 5. Increments and decrements.
    let counter_val = tiered
        .increment("count", 5, Duration::from_secs(3600))
        .await
        .unwrap();
    assert_eq!(counter_val, 5);
    let decr_val = tiered
        .decrement("count", 2, Duration::from_secs(3600))
        .await
        .unwrap();
    assert_eq!(decr_val, 3);

    // 6. Pull and forget many.
    tiered
        .put("pull_k", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    let pulled = tiered
        .pull("pull_k")
        .await
        .unwrap()
        .expect("pulled from tiered");
    assert_eq!(pulled.deserialize::<String>().unwrap(), "tiered_value");

    tiered
        .put("m1", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    tiered
        .put("m2", &entry, Duration::from_secs(3600))
        .await
        .unwrap();
    assert_eq!(tiered.forget_many(&["m1", "m2"]).await.unwrap(), 2);

    // 7. Accessors.
    assert_eq!(tiered.memory().len(), 1); // item2 is still present
    assert!(
        tiered
            .distributed()
            .get("nonexistent")
            .await
            .unwrap()
            .is_none()
    );

    // 8. Flush flushes both L1 and L2.
    tiered.flush().await.unwrap();
    assert_eq!(tiered.memory().len(), 0);
    assert!(tiered.get("count").await.unwrap().is_none());
    assert!(memory.get("count").await.unwrap().is_none());
}

#[tokio::test]
async fn test_tiered_cache_store_fallback_insurance() {
    let memory = MemoryCacheStore::new();
    let null_store = Arc::new(cache::NullCacheStore::new());

    // Tiered with Null as L2 simulating missing remote entries.
    let tiered = TieredCacheStore::new(memory.clone(), null_store)
        .with_fallback_insurance(true)
        .with_short_circuit(true);

    let entry = CacheEntry::from_value(
        &"local_saved",
        Duration::from_secs(3600),
        Duration::ZERO,
        Vec::new(),
    )
    .unwrap();
    tiered
        .put("k", &entry, Duration::from_secs(3600))
        .await
        .unwrap();

    let fetched = tiered.get("k").await.unwrap().expect("served from L1");
    assert_eq!(fetched.deserialize::<String>().unwrap(), "local_saved");
}
