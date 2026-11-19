//! Lightweight timestamp-vector tag invalidation system.

use std::time::Duration;

use crate::{
    entry::{CacheEntry, current_unix_timestamp_ms},
    error::CacheError,
    store::CacheStore,
};

/// Key prefix for tag invalidation timestamp records.
pub const TAG_KEY_PREFIX: &str = "__druto_tag:";

/// Formats the storage key used to track invalidation timestamps for a given tag.
#[must_use]
pub fn format_tag_key(tag: &str) -> String {
    format!("{TAG_KEY_PREFIX}{tag}")
}

/// Checks if a storage key is an internal tag invalidation timestamp key.
#[must_use]
pub fn is_tag_key(key: &str) -> bool {
    key.starts_with(TAG_KEY_PREFIX)
}

/// Invalidates all cache entries associated with the specified tags.
///
/// Writes a monotonic timestamp onto each tag's tracking key with a generous expiration duration
/// (defaulting to 30 days if omitted) so that entries created prior to this timestamp are rejected.
///
/// # Errors
///
/// Returns [`CacheError`] if storing the tag timestamp fails.
pub async fn invalidate_tags(
    tags: &[&str],
    store: &dyn CacheStore,
    tag_record_ttl: Duration,
) -> Result<(), CacheError> {
    let now_ms = current_unix_timestamp_ms();

    for tag in tags {
        let key = format_tag_key(tag);
        let entry = CacheEntry::from_value(&now_ms, tag_record_ttl, Duration::ZERO, Vec::new())?;
        store.put(&key, &entry, tag_record_ttl).await?;
    }

    Ok(())
}

/// Evaluates whether a [`CacheEntry`] remains valid against all its associated tags.
///
/// An entry is valid if and only if its `created_at_ms` timestamp is strictly greater
/// than the invalidation timestamp of every tag associated with it.
///
/// # Errors
///
/// Returns [`CacheError`] if querying the tag timestamp fails.
pub async fn is_entry_tag_valid(
    entry: &CacheEntry,
    store: &dyn CacheStore,
) -> Result<bool, CacheError> {
    if entry.tags.is_empty() {
        return Ok(true);
    }

    // Do not check tag invalidation on tag keys themselves.
    if is_tag_key(&entry.value) {
        return Ok(true);
    }

    for tag in &entry.tags {
        let key = format_tag_key(tag);
        let tag_entry = store.get(&key).await?;
        let is_invalid = tag_entry
            .and_then(|t| t.deserialize::<u64>().ok())
            .is_some_and(|invalidated_at_ms| entry.created_at_ms <= invalidated_at_ms);

        if is_invalid {
            return Ok(false);
        }
    }

    Ok(true)
}
