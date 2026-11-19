//! Configuration types and defaults for cache stores and manager registries.

use std::{collections::HashMap, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

/// Default global cache TTL (1 hour).
pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Driver configuration options for individual storage backends.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "driver", rename_all = "lowercase")]
pub enum StoreConfig {
    /// In-memory concurrent cache.
    Memory {
        /// Optional maximum capacity of entries before eviction occurs.
        max_capacity: Option<usize>,
    },
    /// Local filesystem persistence driver.
    File {
        /// Target directory where JSON cache files are persisted.
        directory: PathBuf,
    },
    /// Distributed Redis cache driver.
    #[cfg(feature = "redis")]
    Redis {
        /// Key prefix for cache entries (defaults to `druto:cache:`).
        prefix: Option<String>,
    },
    /// Hybrid tiered cache driver combining in-memory L1 and distributed L2.
    Tiered {
        /// Name of the configured memory store to use as L1.
        memory_store: String,
        /// Name of the configured distributed store to use as L2.
        distributed_store: String,
        /// Whether to degrade to L1 memory if L2 distributed store encounters errors.
        fallback_insurance: Option<bool>,
        /// Whether to short-circuit reads from L1 memory before querying L2.
        short_circuit: Option<bool>,
    },
    /// Zero-allocation, no-op cache.
    Null,
}

/// Global framework caching configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// The name of the default cache store to use when none is explicitly requested.
    pub default: String,
    /// Default time-to-live expiration applied to stored entries.
    pub default_ttl: Duration,
    /// Default grace period window for stale-while-revalidate serving.
    pub default_grace_period: Duration,
    /// Registry of named cache store configurations.
    pub stores: HashMap<String, StoreConfig>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let mut stores = HashMap::new();
        stores.insert(
            "memory".to_string(),
            StoreConfig::Memory {
                max_capacity: Some(10_000),
            },
        );
        stores.insert("null".to_string(), StoreConfig::Null);

        Self {
            default: "memory".to_string(),
            default_ttl: DEFAULT_CACHE_TTL,
            default_grace_period: Duration::ZERO,
            stores,
        }
    }
}

impl CacheConfig {
    /// Creates a new cache configuration with the given default store name.
    #[must_use]
    pub fn new(default_store: impl Into<String>) -> Self {
        Self {
            default: default_store.into(),
            ..Default::default()
        }
    }

    /// Sets the global default TTL duration.
    #[must_use]
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Sets the global default grace period duration.
    #[must_use]
    pub fn with_default_grace_period(mut self, grace: Duration) -> Self {
        self.default_grace_period = grace;
        self
    }

    /// Adds a named store configuration to the registry.
    pub fn add_store(&mut self, name: impl Into<String>, config: StoreConfig) {
        self.stores.insert(name.into(), config);
    }
}
