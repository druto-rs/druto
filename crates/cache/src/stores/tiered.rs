//! Hybrid multi-tier cache store coordinating fast in-memory L1 and distributed L2.

use std::{sync::Arc, time::Duration};

use crate::{
    entry::{CacheEntry, current_unix_timestamp_ms},
    error::CacheError,
    store::{BoxFuture, CacheStore},
    stores::memory::MemoryCacheStore,
};

/// Hybrid two-tier cache store coordinating fast in-memory L1 and distributed L2.
#[derive(Clone)]
pub struct TieredCacheStore {
    memory: Arc<MemoryCacheStore>,
    distributed: Arc<dyn CacheStore>,
    fallback_insurance: bool,
    short_circuit: bool,
}

impl TieredCacheStore {
    /// Constructs a new `TieredCacheStore` coordinating L1 memory and L2 distributed storage.
    pub fn new(memory: MemoryCacheStore, distributed: Arc<dyn CacheStore>) -> Self {
        Self {
            memory: Arc::new(memory),
            distributed,
            fallback_insurance: true,
            short_circuit: true,
        }
    }

    /// Toggles fallback insurance (degrades to L1 in-memory caching if L2 distributed store fails).
    #[must_use]
    pub fn with_fallback_insurance(mut self, enabled: bool) -> Self {
        self.fallback_insurance = enabled;
        self
    }

    /// Toggles short-circuit reads from L1 memory before querying L2.
    #[must_use]
    pub fn with_short_circuit(mut self, enabled: bool) -> Self {
        self.short_circuit = enabled;
        self
    }

    /// Returns a reference to the L1 memory store.
    #[must_use]
    pub fn memory(&self) -> &MemoryCacheStore {
        &self.memory
    }

    /// Returns a reference to the L2 distributed store.
    #[must_use]
    pub fn distributed(&self) -> &Arc<dyn CacheStore> {
        &self.distributed
    }
}

impl CacheStore for TieredCacheStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            // 1. Check L1 in-memory fast cache first if short-circuit is enabled.
            if self.short_circuit {
                let local_entry = self.memory.get(key).await?;
                if local_entry.is_some() {
                    return Ok(local_entry);
                }
            }

            // 2. Query L2 distributed store.
            let distributed_result = self.distributed.get(key).await;

            match distributed_result {
                Ok(Some(entry)) => {
                    // Backfill L1 memory with the entry using remaining physical TTL.
                    let now_ms = current_unix_timestamp_ms();
                    let remaining_ms = entry.physical_expiration_ms.saturating_sub(now_ms);
                    if remaining_ms > 0 {
                        let remaining_ttl = Duration::from_millis(remaining_ms);
                        let backfill_result = self.memory.put(key, &entry, remaining_ttl).await;
                        drop(backfill_result);
                    }
                    Ok(Some(entry))
                }
                Ok(None) => Ok(None),
                Err(error) => {
                    if self.fallback_insurance {
                        // Degrade gracefully to whatever L1 memory holds.
                        self.memory.get(key).await
                    } else {
                        Err(error)
                    }
                }
            }
        })
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CacheError>> {
        Box::pin(async move {
            // Write to L2 distributed store first.
            let distributed_result = self.distributed.put(key, entry, ttl).await;

            if let Err(error) = distributed_result {
                if self.fallback_insurance {
                    // Still populate local L1 cache under fallback insurance.
                    let local_result = self.memory.put(key, entry, ttl).await;
                    drop(local_result);
                    return Ok(());
                }
                return Err(error);
            }

            // Write to L1 memory cache.
            self.memory.put(key, entry, ttl).await?;
            Ok(())
        })
    }

    fn add<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let distributed_result = self.distributed.add(key, entry, ttl).await;

            match distributed_result {
                Ok(true) => {
                    let local_result = self.memory.put(key, entry, ttl).await;
                    drop(local_result);
                    Ok(true)
                }
                Ok(false) => Ok(false),
                Err(error) => {
                    if self.fallback_insurance {
                        self.memory.add(key, entry, ttl).await
                    } else {
                        Err(error)
                    }
                }
            }
        })
    }

    fn pull<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let l1_remove = self.memory.forget(key).await;
            drop(l1_remove);
            self.distributed.pull(key).await
        })
    }

    fn forget<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let l1_remove = self.memory.forget(key).await;
            drop(l1_remove);
            self.distributed.forget(key).await
        })
    }

    fn forget_many<'a>(&'a self, keys: &'a [&'a str]) -> BoxFuture<'a, Result<usize, CacheError>> {
        Box::pin(async move {
            let l1_remove = self.memory.forget_many(keys).await;
            drop(l1_remove);
            self.distributed.forget_many(keys).await
        })
    }

    fn increment<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>> {
        Box::pin(async move {
            // Invalidate local memory to avoid stale counter reads.
            let l1_remove = self.memory.forget(key).await;
            drop(l1_remove);
            let distributed_result = self.distributed.increment(key, amount, ttl).await;

            match distributed_result {
                Ok(val) => Ok(val),
                Err(error) => {
                    if self.fallback_insurance {
                        self.memory.increment(key, amount, ttl).await
                    } else {
                        Err(error)
                    }
                }
            }
        })
    }

    fn flush(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move {
            let l1_flush = self.memory.flush().await;
            drop(l1_flush);
            self.distributed.flush().await
        })
    }
}
