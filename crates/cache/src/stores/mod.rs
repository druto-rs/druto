//! Storage backend drivers for in-memory, distributed, tiered, and filesystem caching.

pub mod file;
pub mod memory;
pub mod null;
#[cfg(feature = "redis")]
pub mod redis;
pub mod tiered;

pub use file::FileCacheStore;
pub use memory::MemoryCacheStore;
pub use null::NullCacheStore;
#[cfg(feature = "redis")]
pub use redis::RedisCacheStore;
pub use tiered::TieredCacheStore;
