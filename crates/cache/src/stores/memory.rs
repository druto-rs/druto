//! High-concurrency sharded in-memory cache store with bounded capacity and TTL/LRU eviction.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    entry::{CacheEntry, current_unix_timestamp_ms},
    error::CacheError,
    store::{BoxFuture, CacheStore},
};

const NUM_SHARDS: usize = 16;
const DEFAULT_CAPACITY: usize = 10_000;

#[derive(Default)]
struct MemoryShard {
    entries: RwLock<HashMap<String, CacheEntry>>,
}

/// Thread-safe in-memory cache store using sharded locks to maximize multi-core throughput.
#[derive(Clone)]
pub struct MemoryCacheStore {
    shards: Arc<[MemoryShard; NUM_SHARDS]>,
    max_capacity: usize,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}

impl Default for MemoryCacheStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCacheStore {
    /// Creates a new in-memory cache store with default capacity (10,000 entries).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Creates a new in-memory cache store with a specified maximum capacity.
    #[must_use]
    pub fn with_capacity(max_capacity: usize) -> Self {
        let shards = std::array::from_fn(|_| MemoryShard::default());
        Self {
            shards: Arc::new(shards),
            max_capacity: max_capacity.max(NUM_SHARDS),
            clock: Arc::new(current_unix_timestamp_ms),
        }
    }

    /// Creates an in-memory store with an injected clock function for deterministic testing.
    #[must_use]
    pub fn with_clock<F>(max_capacity: usize, clock: F) -> Self
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        let shards = std::array::from_fn(|_| MemoryShard::default());
        Self {
            shards: Arc::new(shards),
            max_capacity: max_capacity.max(NUM_SHARDS),
            clock: Arc::new(clock),
        }
    }

    /// Returns the maximum total entry capacity across all shards.
    #[must_use]
    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    /// Returns the total number of entries currently stored across all shards.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|shard| {
                shard
                    .entries
                    .read()
                    .map(|entries| entries.len())
                    .unwrap_or_default()
            })
            .sum()
    }

    /// Checks if the store is currently empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn now_ms(&self) -> u64 {
        (self.clock)()
    }

    fn shard_for(&self, key: &str) -> &MemoryShard {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let index = (hasher.finish() as usize) % NUM_SHARDS;
        &self.shards[index]
    }

    fn max_shard_capacity(&self) -> usize {
        self.max_capacity / NUM_SHARDS
    }

    fn evict_if_needed(&self, map: &mut HashMap<String, CacheEntry>, now_ms: u64) {
        let limit = self.max_shard_capacity();
        if map.len() < limit {
            return;
        }

        // 1. Purge physically expired entries.
        map.retain(|_, entry| !entry.is_physically_expired(now_ms));

        // 2. If still at capacity, evict the oldest created entries (oldest 20%).
        if map.len() >= limit {
            let mut items: Vec<(String, u64)> = map
                .iter()
                .map(|(key, entry)| (key.clone(), entry.created_at_ms))
                .collect();
            items.sort_unstable_by_key(|(_, timestamp)| *timestamp);

            let eviction_count = (map.len() / 5).max(1);
            for (key, _) in items.into_iter().take(eviction_count) {
                map.remove(&key);
            }
        }
    }
}

impl CacheStore for MemoryCacheStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let shard = self.shard_for(key);
            let map = shard.entries.read().map_err(|_| {
                CacheError::LockPoisoned("Memory store read lock poisoned".to_string())
            })?;

            if let Some(entry) = map.get(key) {
                if entry.is_physically_expired(self.now_ms()) {
                    Ok(None)
                } else {
                    Ok(Some(entry.clone()))
                }
            } else {
                Ok(None)
            }
        })
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CacheError>> {
        let _ = ttl;
        Box::pin(async move {
            let shard = self.shard_for(key);
            let mut map = shard.entries.write().map_err(|_| {
                CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
            })?;

            self.evict_if_needed(&mut map, self.now_ms());
            map.insert(key.to_string(), entry.clone());
            Ok(())
        })
    }

    fn add<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<bool, CacheError>> {
        let _ = ttl;
        Box::pin(async move {
            let shard = self.shard_for(key);
            let mut map = shard.entries.write().map_err(|_| {
                CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
            })?;

            let now_ms = self.now_ms();
            let is_present_and_unexpired = map
                .get(key)
                .is_some_and(|existing| !existing.is_physically_expired(now_ms));

            if is_present_and_unexpired {
                return Ok(false);
            }

            self.evict_if_needed(&mut map, now_ms);
            map.insert(key.to_string(), entry.clone());
            Ok(true)
        })
    }

    fn pull<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let shard = self.shard_for(key);
            let mut map = shard.entries.write().map_err(|_| {
                CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
            })?;

            let now_ms = self.now_ms();
            if let Some(entry) = map.remove(key) {
                if entry.is_physically_expired(now_ms) {
                    Ok(None)
                } else {
                    Ok(Some(entry))
                }
            } else {
                Ok(None)
            }
        })
    }

    fn forget<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let shard = self.shard_for(key);
            let mut map = shard.entries.write().map_err(|_| {
                CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
            })?;

            Ok(map.remove(key).is_some())
        })
    }

    fn forget_many<'a>(&'a self, keys: &'a [&'a str]) -> BoxFuture<'a, Result<usize, CacheError>> {
        Box::pin(async move {
            let mut removed_count = 0;
            for key in keys {
                let shard = self.shard_for(key);
                let mut map = shard.entries.write().map_err(|_| {
                    CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
                })?;
                if map.remove(*key).is_some() {
                    removed_count += 1;
                }
            }
            Ok(removed_count)
        })
    }

    fn increment<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>> {
        Box::pin(async move {
            let shard = self.shard_for(key);
            let mut map = shard.entries.write().map_err(|_| {
                CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
            })?;

            let now_ms = self.now_ms();
            let current_value = if let Some(existing) = map.get(key) {
                if existing.is_physically_expired(now_ms) {
                    0
                } else {
                    existing.deserialize::<i64>().unwrap_or(0)
                }
            } else {
                0
            };

            let new_value = current_value.saturating_add(amount);
            let entry = CacheEntry::from_value_with_clock(
                &new_value,
                ttl,
                Duration::ZERO,
                Vec::new(),
                now_ms,
            )?;

            self.evict_if_needed(&mut map, now_ms);
            map.insert(key.to_string(), entry);
            Ok(new_value)
        })
    }

    fn flush(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move {
            for shard in self.shards.iter() {
                let mut map = shard.entries.write().map_err(|_| {
                    CacheError::LockPoisoned("Memory store write lock poisoned".to_string())
                })?;
                map.clear();
            }
            Ok(())
        })
    }
}
