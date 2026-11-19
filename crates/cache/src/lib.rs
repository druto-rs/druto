//! Unified multi-driver caching engine for the Druto web framework.
//!
//! Provides in-memory, distributed, and multi-tier caching abstractions with
//! single-flight stampede defense, soft TTL grace periods, and tag-based invalidation.

#![doc(html_root_url = "https://docs.rs/druto-cache/0.1.0")]

pub mod cache;
pub mod config;
pub mod entry;
pub mod error;
pub mod events;
pub mod manager;
pub mod singleflight;
pub mod store;
pub mod stores;
pub mod tag;

pub use cache::{Cache, TaggedCache};
pub use config::{CacheConfig, DEFAULT_CACHE_TTL, StoreConfig};
pub use entry::{CacheEntry, current_unix_timestamp_ms};
pub use error::CacheError;
pub use events::{CacheEvent, CacheEventHandler, CacheTier};
pub use manager::CacheManager;
pub use singleflight::SingleFlight;
pub use store::{BoxFuture, CacheStore};
pub use stores::{FileCacheStore, MemoryCacheStore, NullCacheStore, TieredCacheStore};
#[cfg(feature = "redis")]
pub use stores::{RedisCacheStore, redis::DEFAULT_REDIS_KEY_PREFIX};
pub use tag::{format_tag_key, invalidate_tags, is_entry_tag_valid, is_tag_key};
