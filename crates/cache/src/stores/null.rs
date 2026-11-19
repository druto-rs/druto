//! Zero-allocation no-op cache store for testing and selectively disabling cache layers.

use std::time::Duration;

use crate::{
    entry::CacheEntry,
    error::CacheError,
    store::{BoxFuture, CacheStore},
};

/// Zero-allocation, no-op cache store where all reads miss and all writes succeed.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullCacheStore;

impl NullCacheStore {
    /// Creates a new `NullCacheStore`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl CacheStore for NullCacheStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        let _ = key;
        Box::pin(async move { Ok(None) })
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CacheError>> {
        let _ = (key, entry, ttl);
        Box::pin(async move { Ok(()) })
    }

    fn add<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<bool, CacheError>> {
        let _ = (key, entry, ttl);
        Box::pin(async move { Ok(false) })
    }

    fn pull<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        let _ = key;
        Box::pin(async move { Ok(None) })
    }

    fn forget<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, CacheError>> {
        let _ = key;
        Box::pin(async move { Ok(false) })
    }

    fn forget_many<'a>(&'a self, keys: &'a [&'a str]) -> BoxFuture<'a, Result<usize, CacheError>> {
        let _ = keys;
        Box::pin(async move { Ok(0) })
    }

    fn increment<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>> {
        let _ = (key, ttl);
        Box::pin(async move { Ok(amount) })
    }

    fn flush(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move { Ok(()) })
    }
}
