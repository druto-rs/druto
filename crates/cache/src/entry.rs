//! Cache entry models, expiration metadata, and serialization envelopes.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::error::CacheError;

/// Returns the current Unix timestamp in milliseconds.
#[must_use]
pub fn current_unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Represents a cached entry containing the serialized payload and expiration metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Serialized JSON string payload.
    pub value: String,
    /// Unix millisecond timestamp when the entry was created or updated.
    pub created_at_ms: u64,
    /// Unix millisecond timestamp when the entry logically expires.
    pub logical_expiration_ms: u64,
    /// Unix millisecond timestamp when the entry physically expires (logical + grace).
    pub physical_expiration_ms: u64,
    /// Associated cache tags for targeted invalidation.
    pub tags: Vec<String>,
}

impl CacheEntry {
    /// Constructs a new cache entry from a serializable value with TTL, grace period, and tags.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Serialization`] if the value cannot be serialized into JSON.
    pub fn from_value<T: Serialize>(
        value: &T,
        ttl: Duration,
        grace: Duration,
        tags: Vec<String>,
    ) -> Result<Self, CacheError> {
        let serialized = serde_json::to_string(value)
            .map_err(|error| CacheError::Serialization(error.to_string()))?;
        let now_ms = current_unix_timestamp_ms();
        let logical_ms = now_ms.saturating_add(ttl.as_millis() as u64);
        let physical_ms = logical_ms.saturating_add(grace.as_millis() as u64);

        Ok(Self {
            value: serialized,
            created_at_ms: now_ms,
            logical_expiration_ms: logical_ms,
            physical_expiration_ms: physical_ms,
            tags,
        })
    }

    /// Constructs a cache entry using an injected clock timestamp for deterministic testing.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Serialization`] if the value cannot be serialized into JSON.
    pub fn from_value_with_clock<T: Serialize>(
        value: &T,
        ttl: Duration,
        grace: Duration,
        tags: Vec<String>,
        now_ms: u64,
    ) -> Result<Self, CacheError> {
        let serialized = serde_json::to_string(value)
            .map_err(|error| CacheError::Serialization(error.to_string()))?;
        let logical_ms = now_ms.saturating_add(ttl.as_millis() as u64);
        let physical_ms = logical_ms.saturating_add(grace.as_millis() as u64);

        Ok(Self {
            value: serialized,
            created_at_ms: now_ms,
            logical_expiration_ms: logical_ms,
            physical_expiration_ms: physical_ms,
            tags,
        })
    }

    /// Deserializes the stored JSON payload into the requested type `T`.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Deserialization`] if deserialization fails.
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, CacheError> {
        serde_json::from_str(&self.value)
            .map_err(|error| CacheError::Deserialization(error.to_string()))
    }

    /// Checks if the entry has passed its logical freshness expiration.
    #[must_use]
    pub fn is_logically_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.logical_expiration_ms
    }

    /// Checks if the entry has passed its physical storage expiration (including grace period).
    #[must_use]
    pub fn is_physically_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.physical_expiration_ms
    }

    /// Checks if the entry is within its grace period (logically expired, but not physically expired).
    #[must_use]
    pub fn is_in_grace_period(&self, now_ms: u64) -> bool {
        now_ms >= self.logical_expiration_ms && now_ms < self.physical_expiration_ms
    }

    /// Serializes the entire entry into a storage string (e.g. for Redis or local file persistence).
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Serialization`] if serialization fails.
    pub fn to_storage_string(&self) -> Result<String, CacheError> {
        serde_json::to_string(self).map_err(|error| CacheError::Serialization(error.to_string()))
    }

    /// Deserializes a storage string back into a [`CacheEntry`].
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Deserialization`] if deserialization fails.
    pub fn from_storage_str(input: &str) -> Result<Self, CacheError> {
        serde_json::from_str(input).map_err(|error| CacheError::Deserialization(error.to_string()))
    }
}
