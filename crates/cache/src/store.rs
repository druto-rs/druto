//! Asynchronous cache store trait definition.

use std::{future::Future, pin::Pin, time::Duration};

use crate::{entry::CacheEntry, error::CacheError};

/// A pinned, heap-allocated future that is thread-safe across executors.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Storage backend contract for persisting and retrieving cache entries.
///
/// Implementations must be thread-safe (`Send + Sync + 'static`) and runtime-agnostic.
pub trait CacheStore: Send + Sync + 'static {
    /// Retrieves a cache entry by its key.
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>>;

    /// Stores a cache entry under the given key with a specified physical TTL.
    fn put<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CacheError>>;

    /// Stores a cache entry only if the key does not already exist (or is expired).
    /// Returns `true` if added, `false` otherwise.
    fn add<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<bool, CacheError>>;

    /// Retrieves and atomically removes a cache entry by its key.
    fn pull<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>>;

    /// Removes a cache entry by its key. Returns `true` if removed, `false` if not found.
    fn forget<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, CacheError>>;

    /// Removes multiple cache entries by key. Returns the count of removed entries.
    fn forget_many<'a>(&'a self, keys: &'a [&'a str]) -> BoxFuture<'a, Result<usize, CacheError>>;

    /// Increments the numeric value of a cached integer key by `amount`.
    /// If the key does not exist, it initializes it to `amount`.
    fn increment<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>>;

    /// Decrements the numeric value of a cached integer key by `amount`.
    /// If the key does not exist, it initializes it to `-amount`.
    fn decrement<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>> {
        self.increment(key, -amount, ttl)
    }

    /// Clears all entries from this cache store.
    fn flush(&self) -> BoxFuture<'_, Result<(), CacheError>>;
}
