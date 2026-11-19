use std::{sync::Arc, time::Duration};

use cache::{Cache, CacheConfig, CacheError, CacheManager, MemoryCacheStore, StoreConfig};

#[tokio::test]
async fn test_cache_manager_named_stores_and_default() {
    let mut config = CacheConfig::default();
    config.add_store(
        "secondary",
        StoreConfig::Memory {
            max_capacity: Some(500),
        },
    );

    let manager = CacheManager::new(config);

    // Default store (memory) CRUD via manager delegation.
    manager
        .put("test_key", &"manager_val", Duration::from_secs(60))
        .await
        .unwrap();
    let val: String = manager
        .get("test_key")
        .await
        .unwrap()
        .expect("value exists");
    assert_eq!(val, "manager_val");

    // Named store lookup.
    let secondary = manager.store("secondary").unwrap();
    assert!(secondary.get::<String>("test_key").await.unwrap().is_none());

    secondary
        .put("sec_key", &"sec_val", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        secondary.get::<String>("sec_key").await.unwrap().unwrap(),
        "sec_val"
    );

    // Register store explicitly.
    let custom_cache = Arc::new(Cache::new(Arc::new(MemoryCacheStore::new())));
    manager
        .register_store("custom", custom_cache.clone())
        .unwrap();
    assert!(manager.store("custom").is_ok());

    // Non-existent store returns error.
    let err = manager.store("nonexistent").unwrap_err();
    match err {
        CacheError::StoreNotFound(name) => assert_eq!(name, "nonexistent"),
        other => panic!("unexpected error variant: {other:?}"),
    }

    // Flush on default store.
    manager.flush().await.unwrap();
    assert!(manager.get::<String>("test_key").await.unwrap().is_none());

    // Test remember through manager.
    let rem: String = manager
        .remember("rem_key", Duration::from_secs(60), || async {
            Ok::<String, CacheError>("rem_val".to_string())
        })
        .await
        .unwrap();
    assert_eq!(rem, "rem_val");

    // Test forget through manager.
    assert!(manager.forget("rem_key").await.unwrap());
}

#[tokio::test]
async fn test_cache_manager_instantiating_various_drivers() {
    let temp_dir = std::env::temp_dir().join(format!("manager_cache_test_{}", std::process::id()));

    let mut config = CacheConfig::default();
    config.add_store(
        "disk",
        StoreConfig::File {
            directory: temp_dir.clone(),
        },
    );
    config.add_store("blackhole", StoreConfig::Null);
    config.add_store(
        "tiered_store",
        StoreConfig::Tiered {
            memory_store: "memory".to_string(),
            distributed_store: "disk".to_string(),
            fallback_insurance: Some(true),
            short_circuit: Some(true),
        },
    );
    #[cfg(feature = "redis")]
    config.add_store(
        "redis_uninit",
        StoreConfig::Redis {
            prefix: Some("p:".to_string()),
        },
    );

    let manager = CacheManager::new(config);

    // File store instantiation.
    let file_cache = manager.store("disk").unwrap();
    file_cache
        .put("k", &"disk_val", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        file_cache.get::<String>("k").await.unwrap().unwrap(),
        "disk_val"
    );

    // Null store instantiation.
    let null_cache = manager.store("blackhole").unwrap();
    assert!(null_cache.get::<String>("any").await.unwrap().is_none());

    // Tiered store instantiation.
    let tiered_cache = manager.store("tiered_store").unwrap();
    tiered_cache
        .put("t_key", &"tiered_val", Duration::from_secs(60))
        .await
        .unwrap();
    assert_eq!(
        tiered_cache.get::<String>("t_key").await.unwrap().unwrap(),
        "tiered_val"
    );

    #[cfg(feature = "redis")]
    {
        // Uninitialized Redis store from config should return Config error.
        let redis_err = manager.store("redis_uninit").unwrap_err();
        assert!(matches!(redis_err, CacheError::Config(_)));
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
