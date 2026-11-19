//! Hybrid tiered rate limiter store combining fast in-memory caching with distributed backends.

use std::{sync::Arc, time::Duration};

use crate::{
    algorithm::Algorithm,
    error::LimiterError,
    result::RateLimitResult,
    store::{BoxFuture, LimiterStore},
    stores::memory::MemoryLimiterStore,
};

/// Hybrid two-tier rate limiter coordinating L1 in-memory storage and L2 distributed storage.
#[derive(Clone)]
pub struct TieredLimiterStore {
    memory: Arc<MemoryLimiterStore>,
    distributed: Arc<dyn LimiterStore>,
    fallback_insurance: bool,
    in_memory_short_circuit: bool,
}

impl TieredLimiterStore {
    /// Constructs a new `TieredLimiterStore` with both in-memory and distributed layers.
    pub fn new(memory: MemoryLimiterStore, distributed: Arc<dyn LimiterStore>) -> Self {
        Self {
            memory: Arc::new(memory),
            distributed,
            fallback_insurance: true,
            in_memory_short_circuit: true,
        }
    }

    /// Toggles fallback insurance (degrades to in-memory rate limiting if distributed store fails).
    #[must_use]
    pub fn with_fallback_insurance(mut self, enabled: bool) -> Self {
        self.fallback_insurance = enabled;
        self
    }

    /// Toggles in-memory short-circuiting on exhausted or blocked keys.
    #[must_use]
    pub fn with_short_circuit(mut self, enabled: bool) -> Self {
        self.in_memory_short_circuit = enabled;
        self
    }

    /// Returns a reference to the L1 memory store.
    #[must_use]
    pub fn memory(&self) -> &MemoryLimiterStore {
        &self.memory
    }

    /// Returns a reference to the L2 distributed store.
    #[must_use]
    pub fn distributed(&self) -> &Arc<dyn LimiterStore> {
        &self.distributed
    }
}

impl LimiterStore for TieredLimiterStore {
    fn consume<'a>(
        &'a self,
        key: &'a str,
        amount: u64,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            // 1. Check L1 in-memory short-circuit if enabled.
            if self.in_memory_short_circuit {
                let local_peek = self.memory.peek(key, limit, window, algorithm).await?;
                if local_peek.is_exceeded() {
                    return Ok(local_peek);
                }
            }

            // 2. Consume from L2 distributed store.
            match self
                .distributed
                .consume(key, amount, limit, window, algorithm)
                .await
            {
                Ok(result) => {
                    // If exhausted and short-circuit is active, block in L1 for the remaining duration.
                    if self.in_memory_short_circuit && result.is_exceeded() {
                        let block_duration = result.retry_after.unwrap_or(result.reset_after);
                        let _ = self.memory.block(key, block_duration).await;
                    }
                    Ok(result)
                }
                Err(error) => {
                    // Fall back to L1 memory if fallback insurance is enabled.
                    if self.fallback_insurance {
                        self.memory
                            .consume(key, amount, limit, window, algorithm)
                            .await
                    } else {
                        Err(error)
                    }
                }
            }
        })
    }

    fn peek<'a>(
        &'a self,
        key: &'a str,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            if self.in_memory_short_circuit {
                let local_peek = self.memory.peek(key, limit, window, algorithm).await?;
                if local_peek.is_exceeded() {
                    return Ok(local_peek);
                }
            }

            match self.distributed.peek(key, limit, window, algorithm).await {
                Ok(result) => Ok(result),
                Err(error) => {
                    if self.fallback_insurance {
                        self.memory.peek(key, limit, window, algorithm).await
                    } else {
                        Err(error)
                    }
                }
            }
        })
    }

    fn reset<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), LimiterError>> {
        Box::pin(async move {
            let _ = self.memory.reset(key).await;
            self.distributed.reset(key).await
        })
    }

    fn block<'a>(
        &'a self,
        key: &'a str,
        duration: Duration,
    ) -> BoxFuture<'a, Result<(), LimiterError>> {
        Box::pin(async move {
            let _ = self.memory.block(key, duration).await;
            self.distributed.block(key, duration).await
        })
    }
}
