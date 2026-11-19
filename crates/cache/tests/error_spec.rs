use std::error::Error;

use cache::CacheError;

#[test]
fn test_cache_error_display_and_source() {
    let err_ser = CacheError::Serialization("bad json".to_string());
    assert!(format!("{err_ser}").contains("bad json"));

    let err_de = CacheError::Deserialization("missing field".to_string());
    assert!(format!("{err_de}").contains("missing field"));

    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err_io = CacheError::from(io_err);
    assert!(format!("{err_io}").contains("file not found"));
    assert!(err_io.source().is_some());

    let err_store = CacheError::Store("store down".to_string());
    assert!(format!("{err_store}").contains("store down"));
    assert!(err_store.source().is_none());

    let err_unavail = CacheError::StoreUnavailable("timeout".to_string());
    assert!(format!("{err_unavail}").contains("timeout"));

    let err_cfg = CacheError::Config("invalid prefix".to_string());
    assert!(format!("{err_cfg}").contains("invalid prefix"));

    let err_lock = CacheError::LockPoisoned("poison".to_string());
    assert!(format!("{err_lock}").contains("poison"));

    let err_not_found = CacheError::StoreNotFound("missing_store".to_string());
    assert!(format!("{err_not_found}").contains("missing_store"));

    #[cfg(feature = "redis")]
    {
        let redis_err = redis::RedisError::connection("redis bad connection");
        let converted = CacheError::from(redis_err);
        assert!(format!("{converted}").contains("redis bad connection"));
    }
}
