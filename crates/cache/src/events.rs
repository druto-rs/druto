//! Strongly typed cache observability events and telemetry types.

use std::sync::Arc;

/// Storage tier or lifecycle state where a cache hit occurred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheTier {
    /// Hit occurred in L1 local in-memory storage.
    L1,
    /// Hit occurred in L2 distributed storage.
    L2,
    /// Hit occurred on a logically expired entry within its grace period.
    Graced,
}

/// Represents an observability event emitted by the caching engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEvent {
    /// A cache lookup succeeded.
    Hit {
        /// The cache key that produced the hit.
        key: String,
        /// The storage tier where the entry was satisfied.
        tier: CacheTier,
    },
    /// A cache lookup found no entry or an expired entry.
    Miss {
        /// The cache key that missed.
        key: String,
    },
    /// A cache entry was created or overwritten.
    Written {
        /// The cache key that was written.
        key: String,
        /// Any tags associated with the written entry.
        tags: Vec<String>,
    },
    /// A cache entry was forgotten or invalidated.
    Forgotten {
        /// The cache key that was removed.
        key: String,
    },
}

/// Thread-safe event handler callback for telemetry and metrics monitoring.
pub type CacheEventHandler = Arc<dyn Fn(&CacheEvent) + Send + Sync>;
