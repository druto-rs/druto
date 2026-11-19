//! Filesystem cache store driver with atomic persistence and automated expiration cleanup.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{
    entry::{CacheEntry, current_unix_timestamp_ms},
    error::CacheError,
    store::{BoxFuture, CacheStore},
};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Filesystem cache store persisting serialized entries as atomic JSON files.
#[derive(Debug, Clone)]
pub struct FileCacheStore {
    directory: PathBuf,
}

impl FileCacheStore {
    /// Constructs a new filesystem cache store within the given directory.
    ///
    /// Automatically creates the directory tree if it does not yet exist.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Io`] if the directory cannot be created or accessed.
    pub fn new(directory: impl AsRef<Path>) -> Result<Self, CacheError> {
        let path = directory.as_ref().to_path_buf();
        fs::create_dir_all(&path)?;
        Ok(Self { directory: path })
    }

    /// Returns a reference to the storage directory.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Computes a safe, deterministic filename for a given cache key.
    #[must_use]
    pub fn key_file_path(&self, key: &str) -> PathBuf {
        let hex_name: String = key.bytes().map(|byte| format!("{byte:02x}")).collect();
        self.directory.join(format!("{hex_name}.json"))
    }

    /// Sweeps the cache directory, pruning any expired cache entries and stale `.tmp` files.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Io`] if reading directory entries fails.
    pub fn clean_expired(&self) -> Result<usize, CacheError> {
        let now_ms = current_unix_timestamp_ms();
        let entries = fs::read_dir(&self.directory)?;
        let mut pruned_count = 0;

        for entry_result in entries {
            let entry = match entry_result {
                Ok(entry) => entry,
                Err(error) => return Err(CacheError::Io(error)),
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();

            if file_name.starts_with("temp_") && file_name.ends_with(".tmp") {
                let remove_result = fs::remove_file(&path);
                drop(remove_result);
                pruned_count += 1;
                continue;
            }

            if file_name.ends_with(".json") {
                let should_remove = match fs::read_to_string(&path) {
                    Ok(content) => match CacheEntry::from_storage_str(&content) {
                        Ok(cache_entry) => cache_entry.is_physically_expired(now_ms),
                        Err(_) => true,
                    },
                    Err(_) => false,
                };

                if should_remove {
                    let remove_result = fs::remove_file(&path);
                    drop(remove_result);
                    pruned_count += 1;
                }
            }
        }

        Ok(pruned_count)
    }

    fn write_atomic(&self, target_path: &Path, content: &str) -> Result<(), CacheError> {
        let counter = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let now_ms = current_unix_timestamp_ms();
        let tmp_path = self.directory.join(format!(
            "temp_{now_ms}_{counter}_{}.tmp",
            std::process::id()
        ));

        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, target_path)?;
        Ok(())
    }
}

impl CacheStore for FileCacheStore {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let path = self.key_file_path(key);
            if !path.exists() {
                return Ok(None);
            }

            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(CacheError::Io(error)),
            };

            let entry = CacheEntry::from_storage_str(&content)?;
            let now_ms = current_unix_timestamp_ms();

            if entry.is_physically_expired(now_ms) {
                let remove_result = fs::remove_file(&path);
                drop(remove_result);
                Ok(None)
            } else {
                Ok(Some(entry))
            }
        })
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CacheError>> {
        let _ = ttl;
        Box::pin(async move {
            let path = self.key_file_path(key);
            let content = entry.to_storage_string()?;
            self.write_atomic(&path, &content)
        })
    }

    fn add<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let path = self.key_file_path(key);
            if path.exists() {
                let is_unexpired = fs::read_to_string(&path)
                    .ok()
                    .and_then(|content| CacheEntry::from_storage_str(&content).ok())
                    .is_some_and(|existing| {
                        !existing.is_physically_expired(current_unix_timestamp_ms())
                    });

                if is_unexpired {
                    return Ok(false);
                }
            }

            self.put(key, entry, ttl).await?;
            Ok(true)
        })
    }

    fn pull<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let entry = self.get(key).await?;
            if entry.is_some() {
                let remove_result = fs::remove_file(self.key_file_path(key));
                drop(remove_result);
            }
            Ok(entry)
        })
    }

    fn forget<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let path = self.key_file_path(key);
            if path.exists() {
                let remove_result = fs::remove_file(&path);
                Ok(remove_result.is_ok())
            } else {
                Ok(false)
            }
        })
    }

    fn forget_many<'a>(&'a self, keys: &'a [&'a str]) -> BoxFuture<'a, Result<usize, CacheError>> {
        Box::pin(async move {
            let mut removed_count = 0;
            for key in keys {
                if self.forget(key).await? {
                    removed_count += 1;
                }
            }
            Ok(removed_count)
        })
    }

    fn increment<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>> {
        Box::pin(async move {
            let existing = self.get(key).await?;
            let current_value = existing
                .and_then(|entry| entry.deserialize::<i64>().ok())
                .unwrap_or(0);
            let new_value = current_value.saturating_add(amount);

            let entry = CacheEntry::from_value(&new_value, ttl, Duration::ZERO, Vec::new())?;
            self.put(key, &entry, ttl).await?;
            Ok(new_value)
        })
    }

    fn flush(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move {
            let entries = fs::read_dir(&self.directory)?;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let remove_result = fs::remove_file(&path);
                    drop(remove_result);
                }
            }
            Ok(())
        })
    }
}
