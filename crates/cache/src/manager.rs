//! Multi-store cache manager and registry coordinating named drivers.

use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, RwLock},
    time::Duration,
};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    cache::Cache,
    config::{CacheConfig, StoreConfig},
    error::CacheError,
    stores::{FileCacheStore, MemoryCacheStore, NullCacheStore, TieredCacheStore},
};

/// Multi-store cache manager coordinating named cache drivers and default store delegation.
pub struct CacheManager {
    config: CacheConfig,
    instances: Arc<RwLock<HashMap<String, Arc<Cache>>>>,
}

impl CacheManager {
    /// Constructs a new cache manager using the supplied configuration.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Explicitly registers an instantiated [`Cache`] under a store name.
    pub fn register_store(
        &self,
        name: impl Into<String>,
        cache: Arc<Cache>,
    ) -> Result<(), CacheError> {
        let mut map = self.instances.write().map_err(|_| {
            CacheError::LockPoisoned("CacheManager write lock poisoned".to_string())
        })?;
        map.insert(name.into(), cache);
        Ok(())
    }

    /// Retrieves an instantiated cache instance by its configured store name.
    ///
    /// Lazily instantiates the underlying store driver if not yet active.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::StoreNotFound`] if no store is configured with the given name.
    pub fn store(&self, name: &str) -> Result<Arc<Cache>, CacheError> {
        // 1. Fast path: check existing instantiated stores.
        {
            let map = self.instances.read().map_err(|_| {
                CacheError::LockPoisoned("CacheManager read lock poisoned".to_string())
            })?;
            if let Some(instance) = map.get(name) {
                return Ok(Arc::clone(instance));
            }
        }

        // 2. Slow path: lazily instantiate store from configuration.
        let store_config = self
            .config
            .stores
            .get(name)
            .cloned()
            .ok_or_else(|| CacheError::StoreNotFound(name.to_string()))?;

        let cache = Arc::new(self.build_cache_from_config(&store_config)?);

        let mut map = self.instances.write().map_err(|_| {
            CacheError::LockPoisoned("CacheManager write lock poisoned".to_string())
        })?;
        map.insert(name.to_string(), Arc::clone(&cache));

        Ok(cache)
    }

    /// Retrieves the configured default cache store.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if the default store is not found or fails to initialize.
    pub fn default_store(&self) -> Result<Arc<Cache>, CacheError> {
        self.store(&self.config.default)
    }

    fn build_cache_from_config(&self, config: &StoreConfig) -> Result<Cache, CacheError> {
        let cache = match config {
            StoreConfig::Memory { max_capacity } => {
                let capacity = max_capacity.unwrap_or(10_000);
                let store = Arc::new(MemoryCacheStore::with_capacity(capacity));
                Cache::new(store)
            }
            StoreConfig::File { directory } => {
                let store = Arc::new(FileCacheStore::new(directory)?);
                Cache::new(store)
            }
            #[cfg(feature = "redis")]
            StoreConfig::Redis { prefix } => {
                return Err(CacheError::Config(format!(
                    "Redis driver '{prefix:?}' requires live connection initialization; register via `register_store`"
                )));
            }
            StoreConfig::Tiered {
                memory_store,
                distributed_store,
                fallback_insurance,
                short_circuit,
            } => {
                let memory_instance = self.store(memory_store)?;
                let distributed_instance = self.store(distributed_store)?;

                let memory_driver = MemoryCacheStore::new();
                let mut tiered =
                    TieredCacheStore::new(memory_driver, Arc::clone(distributed_instance.store()));

                if let Some(fallback) = fallback_insurance {
                    tiered = tiered.with_fallback_insurance(*fallback);
                }
                if let Some(short_circ) = short_circuit {
                    tiered = tiered.with_short_circuit(*short_circ);
                }

                Cache::new(Arc::new(tiered))
                    .with_default_ttl(memory_instance.default_ttl())
                    .with_grace_period(memory_instance.grace_period())
            }
            StoreConfig::Null => Cache::new(Arc::new(NullCacheStore::new())),
        };

        Ok(cache
            .with_default_ttl(self.config.default_ttl)
            .with_grace_period(self.config.default_grace_period))
    }

    /// Retrieves a value from the default cache store.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if querying storage or deserialization fails.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        self.default_store()?.get(key).await
    }

    /// Stores a value in the default cache store.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or persistence fails.
    pub async fn put<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), CacheError> {
        self.default_store()?.put(key, value, ttl).await
    }

    /// Evaluates or stores a value under single-flight stampede defense on the default store.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if execution or persistence fails.
    pub async fn remember<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        factory: F,
    ) -> Result<T, CacheError>
    where
        T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, CacheError>> + Send + 'static,
    {
        self.default_store()?.remember(key, ttl, factory).await
    }

    /// Removes an entry from the default cache store.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if deletion fails.
    pub async fn forget(&self, key: &str) -> Result<bool, CacheError> {
        self.default_store()?.forget(key).await
    }

    /// Clears all entries from the default cache store.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if clearing storage fails.
    pub async fn flush(&self) -> Result<(), CacheError> {
        self.default_store()?.flush().await
    }
}
