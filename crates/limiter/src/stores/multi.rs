//! Multi-store and multi-dimensional rate limiter combinator.

use std::{sync::Arc, time::Duration};

use crate::{
    algorithm::Algorithm,
    error::LimiterError,
    result::RateLimitResult,
    store::{BoxFuture, LimiterStore},
};

/// A rate limiter bound to a specific key, quota limits, algorithm, and store driver.
#[derive(Clone)]
pub struct BoundLimiter {
    /// Fingerprint key identifier.
    pub key: String,
    /// Maximum allowed requests.
    pub limit: u64,
    /// Time window duration.
    pub window: Duration,
    /// Algorithm for this limiter.
    pub algorithm: Algorithm,
    /// Target store driver.
    pub store: Arc<dyn LimiterStore>,
}

impl BoundLimiter {
    /// Creates a new `BoundLimiter`.
    #[must_use]
    pub fn new(
        key: impl Into<String>,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
        store: Arc<dyn LimiterStore>,
    ) -> Self {
        Self {
            key: key.into(),
            limit,
            window,
            algorithm,
            store,
        }
    }
}

/// Combinator executing rate limiting operations across multiple distinct stores and dimensions.
#[derive(Clone, Default)]
pub struct MultiLimiter {
    limiters: Vec<BoundLimiter>,
}

impl MultiLimiter {
    /// Creates an empty `MultiLimiter`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            limiters: Vec::new(),
        }
    }

    /// Appends a bound limiter to the combinator.
    #[must_use]
    pub fn push_limiter(mut self, limiter: BoundLimiter) -> Self {
        self.limiters.push(limiter);
        self
    }

    /// Consumes quota across all configured limiters, aggregating results.
    pub fn consume(&self) -> BoxFuture<'_, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            if self.limiters.is_empty() {
                return Ok(RateLimitResult::new(
                    u64::MAX,
                    u64::MAX,
                    Duration::from_secs(0),
                    true,
                    None,
                ));
            }

            let mut aggregated: Option<RateLimitResult> = None;

            for bound in &self.limiters {
                let res = bound
                    .store
                    .consume(&bound.key, 1, bound.limit, bound.window, bound.algorithm)
                    .await?;

                aggregated = match aggregated {
                    Some(prev) => Some(prev.combine_most_restrictive(&res)),
                    None => Some(res),
                };
            }

            Ok(aggregated.unwrap())
        })
    }

    /// Peeks status across all limiters without consuming tokens.
    pub fn peek(&self) -> BoxFuture<'_, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            if self.limiters.is_empty() {
                return Ok(RateLimitResult::new(
                    u64::MAX,
                    u64::MAX,
                    Duration::from_secs(0),
                    true,
                    None,
                ));
            }

            let mut aggregated: Option<RateLimitResult> = None;

            for bound in &self.limiters {
                let res = bound
                    .store
                    .peek(&bound.key, bound.limit, bound.window, bound.algorithm)
                    .await?;

                aggregated = match aggregated {
                    Some(prev) => Some(prev.combine_most_restrictive(&res)),
                    None => Some(res),
                };
            }

            Ok(aggregated.unwrap())
        })
    }

    /// Resets all bound limiters.
    pub fn reset(&self) -> BoxFuture<'_, Result<(), LimiterError>> {
        Box::pin(async move {
            for bound in &self.limiters {
                bound.store.reset(&bound.key).await?;
            }
            Ok(())
        })
    }

    /// Blocks all bound limiters for the given penalty duration.
    pub fn block(&self, duration: Duration) -> BoxFuture<'_, Result<(), LimiterError>> {
        Box::pin(async move {
            for bound in &self.limiters {
                bound.store.block(&bound.key, duration).await?;
            }
            Ok(())
        })
    }
}
