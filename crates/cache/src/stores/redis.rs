//! Asynchronous distributed Redis cache store backed by [`RedisCommands`].

use std::{sync::Arc, time::Duration};

use redis::RedisCommands;

use crate::{
    entry::CacheEntry,
    error::CacheError,
    store::{BoxFuture, CacheStore},
};

/// Default key prefix for cache entries stored in Redis.
pub const DEFAULT_REDIS_KEY_PREFIX: &str = "druto:cache:";

/// Asynchronous distributed cache store backed by [`RedisCommands`].
#[derive(Debug, Clone)]
pub struct RedisCacheStore<C> {
    client: Arc<C>,
    prefix: String,
}

impl<C> RedisCacheStore<C> {
    /// Constructs a new Redis cache store using the default key prefix (`druto:cache:`).
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            prefix: DEFAULT_REDIS_KEY_PREFIX.to_string(),
        }
    }

    /// Constructs a new Redis cache store with a custom key prefix.
    pub fn with_prefix(client: Arc<C>, prefix: impl Into<String>) -> Self {
        Self {
            client,
            prefix: prefix.into(),
        }
    }

    /// Returns a reference to the underlying Redis client handle.
    pub fn client(&self) -> &Arc<C> {
        &self.client
    }

    /// Returns the configured key prefix.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Formats the fully qualified storage key for a cache identifier.
    pub fn format_key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }
}

impl<C: RedisCommands + 'static> CacheStore for RedisCacheStore<C> {
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let full_key = self.format_key(key);
            let content = self.client.get(&full_key).await?;

            if let Some(raw) = content {
                let entry = CacheEntry::from_storage_str(&raw)?;
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        })
    }

    fn put<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<(), CacheError>> {
        Box::pin(async move {
            let full_key = self.format_key(key);
            let content = entry.to_storage_string()?;
            let ttl_secs = ttl.as_secs().max(1);

            self.client.set_ex(&full_key, &content, ttl_secs).await?;
            Ok(())
        })
    }

    fn add<'a>(
        &'a self,
        key: &'a str,
        entry: &'a CacheEntry,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let full_key = self.format_key(key);
            let content = entry.to_storage_string()?;
            let added = self.client.set_nx(&full_key, &content).await?;

            if added {
                let ttl_secs = ttl.as_secs().max(1);
                let expire_result = self.client.expire(&full_key, ttl_secs).await;
                drop(expire_result);
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    fn pull<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<CacheEntry>, CacheError>> {
        Box::pin(async move {
            let full_key = self.format_key(key);
            let content = self.client.get(&full_key).await?;

            if let Some(raw) = content {
                let delete_result = self.client.del(&full_key).await;
                drop(delete_result);
                let entry = CacheEntry::from_storage_str(&raw)?;
                Ok(Some(entry))
            } else {
                Ok(None)
            }
        })
    }

    fn forget<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<bool, CacheError>> {
        Box::pin(async move {
            let full_key = self.format_key(key);
            let removed = self.client.del(&full_key).await?;
            Ok(removed)
        })
    }

    fn forget_many<'a>(&'a self, keys: &'a [&'a str]) -> BoxFuture<'a, Result<usize, CacheError>> {
        Box::pin(async move {
            let full_keys: Vec<String> = keys.iter().map(|key| self.format_key(key)).collect();
            let key_refs: Vec<&str> = full_keys.iter().map(AsRef::as_ref).collect();
            let removed = self.client.del_many(&key_refs).await?;
            Ok(removed as usize)
        })
    }

    fn increment<'a>(
        &'a self,
        key: &'a str,
        amount: i64,
        ttl: Duration,
    ) -> BoxFuture<'a, Result<i64, CacheError>> {
        Box::pin(async move {
            let full_key = self.format_key(key);
            let new_value = self.client.incr_by(&full_key, amount).await?;
            if ttl > Duration::ZERO {
                let expire_result = self.client.expire(&full_key, ttl.as_secs().max(1)).await;
                drop(expire_result);
            }
            Ok(new_value)
        })
    }

    fn flush(&self) -> BoxFuture<'_, Result<(), CacheError>> {
        Box::pin(async move {
            let pattern = format!("{}*", self.prefix);
            let keys = self.client.keys(&pattern).await?;

            if !keys.is_empty() {
                let key_refs: Vec<&str> = keys.iter().map(AsRef::as_ref).collect();
                let deleted_count = self.client.del_many(&key_refs).await?;
                let _ = deleted_count;
            }
            Ok(())
        })
    }
}
