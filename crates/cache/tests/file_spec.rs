use std::time::Duration;

use cache::{CacheEntry, CacheStore, FileCacheStore};

#[tokio::test]
async fn test_file_store_crud() {
    let temp_dir = std::env::temp_dir().join(format!("druto_cache_test_{}", std::process::id()));
    let store = FileCacheStore::new(&temp_dir).unwrap();

    let entry = CacheEntry::from_value(
        &"file_data",
        Duration::from_secs(60),
        Duration::ZERO,
        Vec::new(),
    )
    .unwrap();
    store
        .put("my_key", &entry, Duration::from_secs(60))
        .await
        .unwrap();

    let fetched = store
        .get("my_key")
        .await
        .unwrap()
        .expect("file entry found");
    let val: String = fetched.deserialize().unwrap();
    assert_eq!(val, "file_data");

    // Add on existing key.
    assert!(
        !store
            .add("my_key", &entry, Duration::from_secs(60))
            .await
            .unwrap()
    );

    // Pull.
    let pulled = store
        .pull("my_key")
        .await
        .unwrap()
        .expect("pulled file entry");
    assert_eq!(pulled.deserialize::<String>().unwrap(), "file_data");
    assert!(store.get("my_key").await.unwrap().is_none());

    // Increments.
    let count1 = store
        .increment("num", 10, Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(count1, 10);
    let count2 = store
        .decrement("num", 4, Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(count2, 6);

    // Forget & forget many.
    store
        .put("k1", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    store
        .put("k2", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    let removed = store.forget_many(&["k1", "k2"]).await.unwrap();
    assert_eq!(removed, 2);

    // Flush.
    store
        .put("k3", &entry, Duration::from_secs(60))
        .await
        .unwrap();
    store.flush().await.unwrap();
    assert!(store.get("k3").await.unwrap().is_none());

    // Clean expired files, invalid json, and tmp files.
    let stale_tmp = store.directory().join("temp_old.tmp");
    std::fs::write(&stale_tmp, "temporary junk").unwrap();

    let invalid_json = store.directory().join("corrupted.json");
    std::fs::write(&invalid_json, "{not valid json").unwrap();

    let cleaned = store.clean_expired().unwrap();
    assert!(cleaned >= 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
