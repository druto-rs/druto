use std::{sync::Arc, time::Duration};

use cache::{Cache, MemoryCacheStore};

#[tokio::test]
async fn test_tag_invalidation_vectors() {
    let store = Arc::new(MemoryCacheStore::new());
    let cache = Cache::new(store);

    // 1. Store entries with specific tags.
    cache
        .tags(&["users", "org:1"])
        .put("user:1", &"Alice", Duration::from_secs(3600))
        .await
        .unwrap();

    cache
        .tags(&["users", "org:2"])
        .put("user:2", &"Bob", Duration::from_secs(3600))
        .await
        .unwrap();

    cache
        .put("unrelated", &"Standalone", Duration::from_secs(3600))
        .await
        .unwrap();

    assert_eq!(
        cache.get::<String>("user:1").await.unwrap().unwrap(),
        "Alice"
    );
    assert_eq!(cache.get::<String>("user:2").await.unwrap().unwrap(), "Bob");
    assert_eq!(
        cache.get::<String>("unrelated").await.unwrap().unwrap(),
        "Standalone"
    );

    // Small delay to ensure timestamp advancement for invalidation.
    tokio::time::sleep(Duration::from_millis(10)).await;

    // 2. Invalidate only "org:1".
    cache.tags(&["org:1"]).flush().await.unwrap();

    // user:1 should now be invalidated.
    assert!(cache.get::<String>("user:1").await.unwrap().is_none());

    // user:2 and unrelated must still be valid!
    assert_eq!(cache.get::<String>("user:2").await.unwrap().unwrap(), "Bob");
    assert_eq!(
        cache.get::<String>("unrelated").await.unwrap().unwrap(),
        "Standalone"
    );

    tokio::time::sleep(Duration::from_millis(10)).await;

    // 3. Invalidate "users" tag.
    cache.tags(&["users"]).flush().await.unwrap();

    // user:2 should now be invalidated as well.
    assert!(cache.get::<String>("user:2").await.unwrap().is_none());

    // unrelated must still remain unaffected!
    assert_eq!(
        cache.get::<String>("unrelated").await.unwrap().unwrap(),
        "Standalone"
    );
}
