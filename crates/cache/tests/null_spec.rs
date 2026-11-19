use std::time::Duration;

use cache::{CacheEntry, CacheStore, NullCacheStore};

#[tokio::test]
async fn test_null_store_all_methods() {
    let store = NullCacheStore::new();

    let entry = CacheEntry::from_value(
        &"payload",
        Duration::from_secs(60),
        Duration::ZERO,
        Vec::new(),
    )
    .unwrap();

    assert!(store.get("any").await.unwrap().is_none());
    assert!(
        store
            .put("any", &entry, Duration::from_secs(60))
            .await
            .is_ok()
    );
    assert!(
        !store
            .add("any", &entry, Duration::from_secs(60))
            .await
            .unwrap()
    );
    assert!(store.pull("any").await.unwrap().is_none());
    assert!(!store.forget("any").await.unwrap());
    assert_eq!(store.forget_many(&["a", "b"]).await.unwrap(), 0);
    assert_eq!(
        store
            .increment("c", 10, Duration::from_secs(60))
            .await
            .unwrap(),
        10
    );
    assert_eq!(
        store
            .decrement("c", 4, Duration::from_secs(60))
            .await
            .unwrap(),
        -4
    );
    assert!(store.flush().await.is_ok());

    let default_store = NullCacheStore;
    assert!(default_store.get("test").await.unwrap().is_none());
}
