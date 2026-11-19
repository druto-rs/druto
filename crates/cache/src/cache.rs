//! High-level ergonomic cache facade providing strongly typed access, stampede defense, and tags.

use std::{future::Future, sync::Arc, time::Duration};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    entry::{CacheEntry, current_unix_timestamp_ms},
    error::CacheError,
    events::{CacheEvent, CacheEventHandler, CacheTier},
    singleflight::SingleFlight,
    store::CacheStore,
    tag::{invalidate_tags, is_entry_tag_valid},
};

const FOREVER_TTL: Duration = Duration::from_secs(86400 * 365 * 50);

/// Primary high-level caching facade providing generic serialization, single-flight stampede
/// protection, tag namespaces, and stale-while-revalidate serving.
#[derive(Clone)]
pub struct Cache {
    store: Arc<dyn CacheStore>,
    default_ttl: Duration,
    grace_period: Duration,
    prefix: String,
    single_flight: SingleFlight,
    event_handler: Option<CacheEventHandler>,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}

impl std::fmt::Debug for Cache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Cache")
            .field("default_ttl", &self.default_ttl)
            .field("grace_period", &self.grace_period)
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl Cache {
    /// Creates a new cache instance wrapping the underlying storage driver.
    pub fn new(store: Arc<dyn CacheStore>) -> Self {
        Self {
            store,
            default_ttl: Duration::from_secs(3600),
            grace_period: Duration::ZERO,
            prefix: String::new(),
            single_flight: SingleFlight::new(),
            event_handler: None,
            clock: Arc::new(current_unix_timestamp_ms),
        }
    }

    /// Sets the default TTL expiration for keys written through this cache instance.
    #[must_use]
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Sets the default grace period for stale-while-revalidate evaluation.
    #[must_use]
    pub fn with_grace_period(mut self, grace: Duration) -> Self {
        self.grace_period = grace;
        self
    }

    /// Sets a key namespace prefix automatically prepended to all storage keys.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Attaches an observability event handler callback for telemetry and metrics.
    #[must_use]
    pub fn with_events(mut self, handler: CacheEventHandler) -> Self {
        self.event_handler = Some(handler);
        self
    }

    /// Injects a custom clock closure for deterministic time control during tests.
    #[must_use]
    pub fn with_clock<F>(mut self, clock: F) -> Self
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        self.clock = Arc::new(clock);
        self
    }

    /// Returns a reference to the underlying storage backend.
    #[must_use]
    pub fn store(&self) -> &Arc<dyn CacheStore> {
        &self.store
    }

    /// Returns the configured default TTL duration.
    #[must_use]
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// Returns the configured default grace period duration.
    #[must_use]
    pub fn grace_period(&self) -> Duration {
        self.grace_period
    }

    /// Formats the storage key by prepending the configured prefix.
    #[must_use]
    pub fn format_key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }

    fn now_ms(&self) -> u64 {
        (self.clock)()
    }

    fn emit(&self, event: CacheEvent) {
        if let Some(ref handler) = self.event_handler {
            handler(&event);
        }
    }

    /// Retrieves and deserializes a value from the cache.
    ///
    /// Respects tag invalidations, logical expirations, and grace period serving.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if querying storage or deserialization fails.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        let full_key = self.format_key(key);
        let maybe_entry = self.store.get(&full_key).await?;

        if let Some(entry) = maybe_entry {
            let is_valid = is_entry_tag_valid(&entry, self.store.as_ref()).await?;
            if !is_valid {
                self.emit(CacheEvent::Miss {
                    key: key.to_string(),
                });
                return Ok(None);
            }

            let now_ms = self.now_ms();
            if !entry.is_logically_expired(now_ms) {
                self.emit(CacheEvent::Hit {
                    key: key.to_string(),
                    tier: CacheTier::L1,
                });
                let value = entry.deserialize::<T>()?;
                return Ok(Some(value));
            }

            if entry.is_in_grace_period(now_ms) {
                self.emit(CacheEvent::Hit {
                    key: key.to_string(),
                    tier: CacheTier::Graced,
                });
                let value = entry.deserialize::<T>()?;
                return Ok(Some(value));
            }

            self.emit(CacheEvent::Miss {
                key: key.to_string(),
            });
            Ok(None)
        } else {
            self.emit(CacheEvent::Miss {
                key: key.to_string(),
            });
            Ok(None)
        }
    }

    /// Stores a serializable value under the given key with an explicit TTL duration.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or storage persistence fails.
    pub async fn put<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), CacheError> {
        self.put_with_tags(key, value, ttl, Vec::new()).await
    }

    /// Stores a serializable value with explicit tags for targeted invalidation.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or storage persistence fails.
    pub async fn put_with_tags<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
        tags: Vec<String>,
    ) -> Result<(), CacheError> {
        let full_key = self.format_key(key);
        let now_ms = self.now_ms();
        let entry =
            CacheEntry::from_value_with_clock(value, ttl, self.grace_period, tags.clone(), now_ms)?;
        let physical_ttl = ttl.saturating_add(self.grace_period);

        self.store.put(&full_key, &entry, physical_ttl).await?;
        self.emit(CacheEvent::Written {
            key: key.to_string(),
            tags,
        });
        Ok(())
    }

    /// Stores a serializable value indefinitely using a 50-year expiration ceiling.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or storage persistence fails.
    pub async fn put_forever<T: Serialize>(&self, key: &str, value: &T) -> Result<(), CacheError> {
        self.put(key, value, FOREVER_TTL).await
    }

    /// Stores a value only if the key does not already exist (or has physically expired).
    ///
    /// Returns `true` if the item was added, or `false` if it already exists.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or storage persistence fails.
    pub async fn add<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<bool, CacheError> {
        let full_key = self.format_key(key);
        let now_ms = self.now_ms();
        let entry =
            CacheEntry::from_value_with_clock(value, ttl, self.grace_period, Vec::new(), now_ms)?;
        let physical_ttl = ttl.saturating_add(self.grace_period);

        let added = self.store.add(&full_key, &entry, physical_ttl).await?;
        if added {
            self.emit(CacheEvent::Written {
                key: key.to_string(),
                tags: Vec::new(),
            });
        }
        Ok(added)
    }

    /// Retrieves and atomically removes a value from the cache.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if storage query or deserialization fails.
    pub async fn pull<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        let full_key = self.format_key(key);
        let maybe_entry = self.store.pull(&full_key).await?;

        if let Some(entry) = maybe_entry {
            let is_valid = is_entry_tag_valid(&entry, self.store.as_ref()).await?;
            if !is_valid {
                return Ok(None);
            }
            self.emit(CacheEvent::Forgotten {
                key: key.to_string(),
            });
            let value = entry.deserialize::<T>()?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Checks if a key exists in the cache and is currently valid.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if querying storage fails.
    pub async fn has(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.get::<serde_json::Value>(key).await?.is_some())
    }

    /// Checks if a key is absent or expired in the cache.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if querying storage fails.
    pub async fn missing(&self, key: &str) -> Result<bool, CacheError> {
        Ok(!self.has(key).await?)
    }

    /// Removes a key from the cache. Returns `true` if the key was present.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if storage deletion fails.
    pub async fn forget(&self, key: &str) -> Result<bool, CacheError> {
        let full_key = self.format_key(key);
        let removed = self.store.forget(&full_key).await?;
        if removed {
            self.emit(CacheEvent::Forgotten {
                key: key.to_string(),
            });
        }
        Ok(removed)
    }

    /// Removes multiple keys from the cache, returning the count of removed keys.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if storage deletion fails.
    pub async fn forget_many(&self, keys: &[&str]) -> Result<usize, CacheError> {
        let full_keys: Vec<String> = keys.iter().map(|key| self.format_key(key)).collect();
        let key_refs: Vec<&str> = full_keys.iter().map(AsRef::as_ref).collect();
        let count = self.store.forget_many(&key_refs).await?;
        for key in keys {
            self.emit(CacheEvent::Forgotten {
                key: (*key).to_string(),
            });
        }
        Ok(count)
    }

    /// Increments the numeric value of an integer key by `amount`.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if storage update fails.
    pub async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        let full_key = self.format_key(key);
        self.store
            .increment(&full_key, amount, self.default_ttl)
            .await
    }

    /// Decrements the numeric value of an integer key by `amount`.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if storage update fails.
    pub async fn decrement(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        self.increment(key, -amount).await
    }

    /// Flushes all entries from the cache.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if storage flush fails.
    pub async fn flush(&self) -> Result<(), CacheError> {
        self.store.flush().await
    }

    /// Retrieves an item from the cache, or executes `factory` once under single-flight stampede
    /// protection and stores the result for `ttl`.
    ///
    /// If the item is in its grace period, this serves stale data immediately while scheduling
    /// an asynchronous background revalidation.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if computing the value or storage interactions fail.
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
        // 1. Check existing cached entry.
        let full_key = self.format_key(key);
        let maybe_entry = self.store.get(&full_key).await?;

        if let Some(entry) = maybe_entry {
            let is_valid = is_entry_tag_valid(&entry, self.store.as_ref()).await?;
            if is_valid {
                let now_ms = self.now_ms();
                if !entry.is_logically_expired(now_ms) {
                    self.emit(CacheEvent::Hit {
                        key: key.to_string(),
                        tier: CacheTier::L1,
                    });
                    return entry.deserialize::<T>();
                }

                // If in grace period, return stale and revalidate in background.
                if entry.is_in_grace_period(now_ms) {
                    self.emit(CacheEvent::Hit {
                        key: key.to_string(),
                        tier: CacheTier::Graced,
                    });
                    let stale_value = entry.deserialize::<T>()?;

                    // Spawn background revalidation task.
                    let cache_clone = self.clone();
                    let key_string = key.to_string();
                    tokio::spawn(async move {
                        let revalidation_result = cache_clone
                            .execute_factory_and_store(&key_string, ttl, factory)
                            .await;
                        drop(revalidation_result);
                    });

                    return Ok(stale_value);
                }
            }
        }

        // 2. Cache miss: execute under single-flight stampede defense.
        self.execute_factory_and_store(key, ttl, factory).await
    }

    /// Retrieves an item from the cache or stores the factory result permanently.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if computing the value or storage persistence fails.
    pub async fn remember_forever<T, F, Fut>(&self, key: &str, factory: F) -> Result<T, CacheError>
    where
        T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, CacheError>> + Send + 'static,
    {
        self.remember(key, FOREVER_TTL, factory).await
    }

    async fn execute_factory_and_store<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        factory: F,
    ) -> Result<T, CacheError>
    where
        T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, CacheError>>,
    {
        let full_key = self.format_key(key);
        let string_result = self
            .single_flight
            .execute(&full_key, || async {
                let computed = factory().await?;
                let serialized = serde_json::to_string(&computed)
                    .map_err(|error| CacheError::Serialization(error.to_string()))?;
                Ok(serialized)
            })
            .await?;

        let value: T = serde_json::from_str(&string_result)
            .map_err(|error| CacheError::Deserialization(error.to_string()))?;

        self.put(key, &value, ttl).await?;
        Ok(value)
    }

    /// Scopes this cache instance to specific tags.
    #[must_use]
    pub fn tags<'a>(&'a self, tags: &'a [&'a str]) -> TaggedCache<'a> {
        TaggedCache {
            cache: self,
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        }
    }

    /// Scopes this cache to a specific sub-namespace prefix.
    #[must_use]
    pub fn namespace(&self, prefix: &str) -> Self {
        let new_prefix = format!("{}{prefix}:", self.prefix);
        let mut cloned = self.clone();
        cloned.prefix = new_prefix;
        cloned
    }
}

/// A cache wrapper scoped to specific tags for bulk operations and targeted tag invalidation.
pub struct TaggedCache<'a> {
    cache: &'a Cache,
    tags: Vec<String>,
}

impl<'a> TaggedCache<'a> {
    /// Retrieves a value from the cache, respecting the tag context.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if querying storage or deserialization fails.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        self.cache.get(key).await
    }

    /// Stores a value under the scoped tags.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or storage persistence fails.
    pub async fn put<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> Result<(), CacheError> {
        self.cache
            .put_with_tags(key, value, ttl, self.tags.clone())
            .await
    }

    /// Stores a value indefinitely under the scoped tags.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if serialization or storage persistence fails.
    pub async fn put_forever<T: Serialize>(&self, key: &str, value: &T) -> Result<(), CacheError> {
        self.cache
            .put_with_tags(key, value, FOREVER_TTL, self.tags.clone())
            .await
    }

    /// Invalidates all cache entries associated with the tags of this scoped cache.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] if writing the tag invalidation timestamp fails.
    pub async fn flush(&self) -> Result<(), CacheError> {
        let tag_refs: Vec<&str> = self.tags.iter().map(AsRef::as_ref).collect();
        invalidate_tags(
            &tag_refs,
            self.cache.store.as_ref(),
            Duration::from_secs(86400 * 30),
        )
        .await
    }
}
