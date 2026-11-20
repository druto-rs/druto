use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use cache::{Cache, CacheEvent, CacheStore, CacheTier, MemoryCacheStore};

#[tokio::test]
async fn test_cache_facade_features_and_events() {
    let store = Arc::new(MemoryCacheStore::new());

    let recorded_events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&recorded_events);

    let cache = Cache::new(store).with_events(Arc::new(move |event| {
        events_clone.lock().unwrap().push(event.clone());
    }));

    // 1. Put and events.
    cache
        .put("user_name", &"Charlie", Duration::from_secs(3600))
        .await
        .unwrap();

    // 2. Has and missing.
    assert!(cache.has("user_name").await.unwrap());
    assert!(!cache.missing("user_name").await.unwrap());
    assert!(!cache.has("ghost_key").await.unwrap());
    assert!(cache.missing("ghost_key").await.unwrap());

    // 3. Get hit.
    let fetched: String = cache.get("user_name").await.unwrap().unwrap();
    assert_eq!(fetched, "Charlie");

    // 4. Put forever.
    cache.put_forever("immortal", &42).await.unwrap();
    assert_eq!(cache.get::<i32>("immortal").await.unwrap().unwrap(), 42);

    // 5. Remember forever.
    let remembered: String = cache
        .remember_forever("forever_item", || async {
            Ok::<String, cache::CacheError>("eternal".to_string())
        })
        .await
        .unwrap();
    assert_eq!(remembered, "eternal");

    // 6. Forget.
    assert!(cache.forget("user_name").await.unwrap());

    // 7. Pull and add.
    cache
        .put("pull_key", &"pull_me", Duration::from_secs(60))
        .await
        .unwrap();
    let pulled: String = cache.pull("pull_key").await.unwrap().unwrap();
    assert_eq!(pulled, "pull_me");
    assert!(cache.pull::<String>("pull_key").await.unwrap().is_none());

    assert!(
        cache
            .add("add_key", &100, Duration::from_secs(60))
            .await
            .unwrap()
    );
    assert!(
        !cache
            .add("add_key", &200, Duration::from_secs(60))
            .await
            .unwrap()
    );

    // 8. Increments and decrements.
    let inc = cache.increment("num_val", 7).await.unwrap();
    assert_eq!(inc, 7);
    let dec = cache.decrement("num_val", 2).await.unwrap();
    assert_eq!(dec, 5);

    // 9. Forget many and flush.
    cache.put("f1", &1, Duration::from_secs(60)).await.unwrap();
    cache.put("f2", &2, Duration::from_secs(60)).await.unwrap();
    let forgot_count = cache.forget_many(&["f1", "f2"]).await.unwrap();
    assert_eq!(forgot_count, 2);

    cache.flush().await.unwrap();
    assert!(cache.get::<i32>("num_val").await.unwrap().is_none());

    // 10. Tagged cache operations.
    let tagged = cache.tags(&["alpha", "beta"]);
    tagged
        .put_forever("tag_item", &"tagged_payload")
        .await
        .unwrap();
    let fetched_tagged: String = tagged.get("tag_item").await.unwrap().unwrap();
    assert_eq!(fetched_tagged, "tagged_payload");

    // 11. Debug representation & store accessor.
    let debug_str = format!("{cache:?}");
    assert!(debug_str.contains("Cache"));
    assert!(cache.store().get("nonexistent").await.unwrap().is_none());

    // 12. Verify recorded events.
    let events = recorded_events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CacheEvent::Written { key, .. } if key == "user_name"))
    );
    assert!(
        events.iter().any(
            |e| matches!(e, CacheEvent::Hit { key, tier: CacheTier::L1 } if key == "user_name")
        )
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CacheEvent::Miss { key } if key == "ghost_key"))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CacheEvent::Forgotten { key } if key == "user_name"))
    );
}

#[tokio::test]
async fn test_cache_namespace_prefixing() {
    let store = Arc::new(MemoryCacheStore::new());
    let cache = Cache::new(store);

    let scoped = cache.namespace("v1");
    assert_eq!(scoped.format_key("users"), "v1:users");

    scoped
        .put("users", &"UserList", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        scoped.get::<String>("users").await.unwrap().unwrap(),
        "UserList"
    );

    // Root cache does not see "users" without prefix.
    assert!(cache.get::<String>("users").await.unwrap().is_none());
    assert_eq!(
        cache.get::<String>("v1:users").await.unwrap().unwrap(),
        "UserList"
    );
}

#[tokio::test]
async fn test_cache_with_prefix_and_grace_period_hit() {
    let store = Arc::new(MemoryCacheStore::new());
    let recorded_events = Arc::new(Mutex::new(Vec::new()));
    let events_clone = Arc::clone(&recorded_events);

    let cache = Cache::new(store.clone())
        .with_prefix("tenant:")
        .with_events(Arc::new(move |event| {
            events_clone.lock().unwrap().push(event.clone());
        }));

    cache
        .put("settings", &"theme_dark", Duration::from_secs(300))
        .await
        .unwrap();

    let fetched: String = cache.get("settings").await.unwrap().unwrap();
    assert_eq!(fetched, "theme_dark");

    // Check directly in underlying store for prefix
    assert!(store.get("tenant:settings").await.unwrap().is_some());

    // Put a graced entry directly to verify CacheTier::Graced event on hit
    let graced_entry = cache::CacheEntry::from_value(
        &"graced_val",
        Duration::from_millis(10),
        Duration::from_secs(60),
        Vec::new(),
    )
    .unwrap();
    store
        .put("tenant:graced_item", &graced_entry, Duration::from_secs(60))
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(25)).await;
    let graced_fetched: String = cache.get("graced_item").await.unwrap().unwrap();
    assert_eq!(graced_fetched, "graced_val");

    let events = recorded_events.lock().unwrap();
    assert!(events.iter().any(|e| matches!(
        e,
        CacheEvent::Hit {
            key,
            tier: CacheTier::Graced
        } if key == "graced_item"
    )));
}
