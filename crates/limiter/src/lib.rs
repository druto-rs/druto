//! Rate limiting, traffic throttling, burst management, and standardized HTTP response headers for Druto.
//!
//! Provides algorithmic rate limiting engines, pluggable storage backends (including hybrid memory/Redis tiers),
//! and pipeline middleware complying with RFC 6585 and IETF RateLimit header field specifications.

pub mod algorithm;
pub mod config;
pub mod constants;
pub mod error;
pub mod key;
pub mod middleware;
pub mod policy;
pub mod registry;
pub mod result;
pub mod store;
pub mod stores;

pub use algorithm::Algorithm;
pub use config::{
    HeaderMode, LimiterConfig, LimiterConfigBuilder, PolicyResolver, RejectionHandler,
};
pub use constants::{
    RATE_LIMIT_LIMIT, RATE_LIMIT_POLICY, RATE_LIMIT_REMAINING, RATE_LIMIT_RESET, RETRY_AFTER,
    X_RATE_LIMIT_LIMIT, X_RATE_LIMIT_REMAINING, X_RATE_LIMIT_RESET,
};
pub use error::LimiterError;
pub use key::{CustomKeyResolver, KeyResolver};
pub use middleware::{LimiterMiddleware, LimiterMiddlewareBuilder};
pub use policy::{QuotaPolicy, TierConfig};
pub use registry::LimiterRegistry;
pub use result::RateLimitResult;
pub use store::{BoxFuture, LimiterStore};
pub use stores::{BoundLimiter, MemoryLimiterStore, MultiLimiter, TieredLimiterStore};
#[cfg(feature = "redis")]
pub use stores::{DEFAULT_REDIS_KEY_PREFIX, RedisLimiterStore};

/// Convenient prelude re-exporting core rate limiting types and builders.
pub mod prelude {
    #[cfg(feature = "redis")]
    pub use crate::RedisLimiterStore;
    pub use crate::{
        Algorithm, HeaderMode, KeyResolver, LimiterConfig, LimiterConfigBuilder, LimiterError,
        LimiterMiddleware, LimiterRegistry, LimiterStore, MemoryLimiterStore, MultiLimiter,
        QuotaPolicy, RateLimitResult, TierConfig, TieredLimiterStore,
    };
}
