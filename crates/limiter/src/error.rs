//! Strongly typed error types for rate limiting and store drivers.

use std::fmt;

use router::RouterError;

use crate::result::RateLimitResult;

/// Errors produced during rate limiting evaluation, store interaction, or configuration.
#[derive(Debug, Clone)]
pub enum LimiterError {
    /// An error occurred within the underlying storage driver.
    Store(String),
    /// Rate limit has been exceeded for the given key and policy.
    RateLimitExceeded(RateLimitResult),
    /// An invalid configuration was provided to the limiter or policy.
    Config(String),
    /// The backing storage driver is unavailable or timed out.
    StoreUnavailable(String),
}

impl fmt::Display for LimiterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(message) => write!(formatter, "Storage driver error: {message}"),
            Self::RateLimitExceeded(result) => write!(
                formatter,
                "Rate limit exceeded (limit: {}, remaining: 0, reset in {}s)",
                result.limit,
                result.reset_after.as_secs()
            ),
            Self::Config(message) => write!(formatter, "Limiter configuration error: {message}"),
            Self::StoreUnavailable(message) => {
                write!(formatter, "Limiter store unavailable: {message}")
            }
        }
    }
}

impl std::error::Error for LimiterError {}

impl From<LimiterError> for RouterError {
    fn from(error: LimiterError) -> Self {
        match error {
            LimiterError::Store(message) | LimiterError::StoreUnavailable(message) => {
                RouterError::Internal(format!("Limiter error: {message}"))
            }
            LimiterError::Config(message) => {
                RouterError::Internal(format!("Limiter configuration error: {message}"))
            }
            LimiterError::RateLimitExceeded(_) => {
                RouterError::Internal("429 Too Many Requests".to_string())
            }
        }
    }
}
