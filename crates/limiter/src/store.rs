//! Abstract storage driver contracts for rate limiting backends.

use std::{future::Future, pin::Pin, time::Duration};

use crate::{algorithm::Algorithm, error::LimiterError, result::RateLimitResult};

/// Pinned, heap-allocated future for object-safe asynchronous store methods.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Storage backend contract defining atomic operations for rate limiting.
pub trait LimiterStore: Send + Sync + 'static {
    /// Consumes the given amount of quota units for a key, evaluating the algorithm.
    fn consume<'a>(
        &'a self,
        key: &'a str,
        amount: u64,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>>;

    /// Peeks at the current quota status without incrementing or consuming tokens.
    fn peek<'a>(
        &'a self,
        key: &'a str,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>>;

    /// Resets the rate limit state for the given key.
    fn reset<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), LimiterError>>;

    /// Blocks a key for an explicit penalty duration.
    fn block<'a>(
        &'a self,
        key: &'a str,
        duration: Duration,
    ) -> BoxFuture<'a, Result<(), LimiterError>>;
}
