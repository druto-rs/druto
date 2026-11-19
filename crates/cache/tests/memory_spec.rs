use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use cache::{CacheEntry, CacheStore, MemoryCacheStore};

#[tokio::test]
async fn test_memory_store_crud() {
    let store = MemoryCacheStore::new();

    let entry = CacheEntry::from_value(
        &"hello world",
        Duration::from_secs(60),
        Duration::ZERO,
        Vec::new(),
    )
    .unwrap();
    store
        .put("key1", &entry, Duration::from_secs(60))
        .await
        .unwrap();

    let fetched = store
        .get("key1")
        .await
        .unwrap()
        .expect("entry should exist");
    let val: String = fetched.deserialize().unwrap();
    assert_eq!(val, "hello world");

    // Add on existing key should return false.
    let added = store
        .add("key1", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    assert!(!added);

    // Add on new key should return true.
    let added_new = store
        .add("key2", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    assert!(added_new);

    // Pull retrieves and deletes.
    let pulled = store.pull("key2").await.unwrap().expect("pulled entry");
    let pulled_val: String = pulled.deserialize().unwrap();
    assert_eq!(pulled_val, "hello world");
    assert!(store.get("key2").await.unwrap().is_none());

    // Forget removes.
    let removed = store.forget("key1").await.unwrap();
    assert!(removed);
    assert!(!store.forget("key1").await.unwrap());

    // Forget many.
    store
        .put("k1", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    store
        .put("k2", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    let count = store
        .forget_many(&["k1", "k2", "nonexistent"])
        .await
        .unwrap();
    assert_eq!(count, 2);

    // Flush.
    store
        .put("k3", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    store.flush().await.unwrap();
    assert!(store.is_empty());
}

#[tokio::test]
async fn test_memory_store_increments_and_decrements() {
    let store = MemoryCacheStore::new();

    let v1 = store
        .increment("counter", 5, Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(v1, 5);

    let v2 = store
        .increment("counter", 10, Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(v2, 15);

    let v3 = store
        .decrement("counter", 3, Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(v3, 12);
}

#[tokio::test]
async fn test_memory_store_clock_expiration() {
    let clock_time = std::sync::Arc::new(AtomicU64::new(1000));
    let clock_clone = std::sync::Arc::clone(&clock_time);

    let store = MemoryCacheStore::with_clock(1000, move || clock_clone.load(Ordering::Relaxed));

    let entry = CacheEntry::from_value_with_clock(
        &"timed_out",
        Duration::from_millis(500),
        Duration::ZERO,
        Vec::new(),
        1000,
    )
    .unwrap();

    store
        .put("timed", &entry, Duration::from_millis(500))
        .await
        .unwrap();
    assert!(store.get("timed").await.unwrap().is_some());

    // Advance clock past physical expiration.
    clock_time.store(1600, Ordering::Relaxed);
    assert!(store.get("timed").await.unwrap().is_none());

    // Pull on expired returns None.
    assert!(store.pull("timed").await.unwrap().is_none());

    // Add on expired succeeds.
    assert!(
        store
            .add("timed", &entry, Duration::from_millis(500))
            .await
            .unwrap()
    );

    // Increment on expired re-initializes.
    clock_time.store(3000, Ordering::Relaxed);
    let inc = store
        .increment("timed", 10, Duration::from_millis(500))
        .await
        .unwrap();
    assert_eq!(inc, 10);
}

#[tokio::test]
async fn test_memory_store_bounded_eviction() {
    let store = MemoryCacheStore::with_capacity(32);
    assert_eq!(store.max_capacity(), 32);

    for i in 0..100 {
        let entry = CacheEntry::from_value(
            &format!("val_{i}"),
            Duration::from_secs(3600),
            Duration::ZERO,
            Vec::new(),
        )
        .unwrap();
        store
            .put(&format!("key_{i}"), &entry, Duration::from_secs(3600))
            .await
            .unwrap();
    }

    // Capacity must remain bounded.
    assert!(store.len() <= store.max_capacity());
}
