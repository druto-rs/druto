use std::{path::PathBuf, time::Duration};

use cache::{CacheConfig, DEFAULT_CACHE_TTL, StoreConfig};

#[test]
fn test_cache_config_builder_and_defaults() {
    let default_config = CacheConfig::default();
    assert_eq!(default_config.default, "memory");
    assert_eq!(default_config.default_ttl, DEFAULT_CACHE_TTL);
    assert_eq!(default_config.default_grace_period, Duration::ZERO);
    assert!(default_config.stores.contains_key("memory"));
    assert!(default_config.stores.contains_key("null"));

    let custom = CacheConfig::new("file")
        .with_default_ttl(Duration::from_secs(1800))
        .with_default_grace_period(Duration::from_secs(300));

    assert_eq!(custom.default, "file");
    assert_eq!(custom.default_ttl, Duration::from_secs(1800));
    assert_eq!(custom.default_grace_period, Duration::from_secs(300));

    let mut configured = custom;
    configured.add_store(
        "disk",
        StoreConfig::File {
            directory: PathBuf::from("/tmp/cache"),
        },
    );
    assert!(configured.stores.contains_key("disk"));
}
