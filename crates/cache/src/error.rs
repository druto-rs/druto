//! Strongly typed error types for caching operations, store drivers, and serialization.

use std::fmt;

/// Errors produced during cache operations, store interactions, or serialization.
#[derive(Debug)]
pub enum CacheError {
    /// Failure during serialization of cache values.
    Serialization(String),
    /// Failure during deserialization of cache values.
    Deserialization(String),
    /// An I/O error occurred during local persistence.
    Io(std::io::Error),
    /// An error occurred within the underlying storage driver.
    Store(String),
    /// The backing storage driver is unavailable or timed out.
    StoreUnavailable(String),
    /// An invalid configuration was provided to the cache or store.
    Config(String),
    /// A synchronization lock was poisoned.
    LockPoisoned(String),
    /// A required cache store was not found in the registry.
    StoreNotFound(String),
}

impl fmt::Display for CacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(reason) => {
                write!(formatter, "Cache serialization error: {reason}")
            }
            Self::Deserialization(reason) => {
                write!(formatter, "Cache deserialization error: {reason}")
            }
            Self::Io(error) => write!(formatter, "Cache I/O error: {error}"),
            Self::Store(reason) => write!(formatter, "Cache store error: {reason}"),
            Self::StoreUnavailable(reason) => {
                write!(formatter, "Cache store unavailable: {reason}")
            }
            Self::Config(reason) => write!(formatter, "Cache configuration error: {reason}"),
            Self::LockPoisoned(reason) => write!(formatter, "Cache lock poisoned: {reason}"),
            Self::StoreNotFound(name) => {
                write!(formatter, "Cache store '{name}' not found in registry")
            }
        }
    }
}

impl std::error::Error for CacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CacheError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(feature = "redis")]
impl From<redis::RedisError> for CacheError {
    fn from(error: redis::RedisError) -> Self {
        Self::Store(error.to_string())
    }
}
