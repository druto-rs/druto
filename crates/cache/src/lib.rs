//! Unified multi-driver caching engine for the Druto web framework.
//!
//! Re-exports the standalone [`kouta`] caching abstractions and integrates them
//! with Druto defaults and the [`redis`] crate.

#![doc(html_root_url = "https://docs.rs/druto-cache/0.1.0")]

pub use kouta::{
    Cache, CacheConfig, CacheEntry, CacheError, CacheEvent, CacheEventHandler, CacheManager,
    CacheStore, CacheTier, DEFAULT_CACHE_TTL, FileCacheStore, MemoryCacheStore, NullCacheStore,
    SingleFlight, StoreConfig, TaggedCache, TieredCacheStore, current_unix_timestamp_ms,
    format_tag_key, invalidate_tags, is_entry_tag_valid, is_tag_key, store::BoxFuture,
};

pub mod store {
    pub use kouta::store::*;
}

pub mod stores {
    pub use kouta::stores::{FileCacheStore, MemoryCacheStore, NullCacheStore, TieredCacheStore};
    #[cfg(feature = "redis")]
    pub mod redis;
    #[cfg(feature = "redis")]
    pub use redis::{DEFAULT_REDIS_KEY_PREFIX, RedisCacheStore};
}

#[cfg(feature = "redis")]
pub use stores::redis::{DEFAULT_REDIS_KEY_PREFIX, RedisCacheStore};
