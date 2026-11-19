//! Thread-safe in-memory rate limiter store with automated eviction.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    algorithm::Algorithm,
    error::LimiterError,
    result::RateLimitResult,
    store::{BoxFuture, LimiterStore},
};

fn default_current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone)]
enum EntryData {
    SlidingWindow {
        current_window: u64,
        current_count: u64,
        previous_count: u64,
    },
    FixedWindow {
        window_start: u64,
        count: u64,
    },
    TokenBucket {
        tokens: f64,
        last_refill_ms: u64,
    },
}

#[derive(Debug, Clone)]
struct KeyRecord {
    data: EntryData,
    blocked_until_ms: Option<u64>,
    last_accessed_ms: u64,
}

/// In-memory storage backend providing atomic, concurrent rate limiting.
#[derive(Clone)]
pub struct MemoryLimiterStore {
    entries: Arc<RwLock<HashMap<String, KeyRecord>>>,
    max_capacity: usize,
    clock: Arc<dyn Fn() -> u64 + Send + Sync>,
}

impl Default for MemoryLimiterStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryLimiterStore {
    /// Creates a new `MemoryLimiterStore` with default capacity (10,000 entries).
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    /// Creates a new `MemoryLimiterStore` with a specific maximum capacity.
    #[must_use]
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_capacity,
            clock: Arc::new(default_current_time_ms),
        }
    }

    /// Returns the maximum capacity of the in-memory store.
    #[must_use]
    pub fn max_capacity(&self) -> usize {
        self.max_capacity
    }

    /// Creates a store with a deterministic clock closure for testing.
    #[must_use]
    pub fn with_clock<F>(max_capacity: usize, clock: F) -> Self
    where
        F: Fn() -> u64 + Send + Sync + 'static,
    {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_capacity,
            clock: Arc::new(clock),
        }
    }

    fn now_ms(&self) -> u64 {
        (self.clock)()
    }

    fn evict_if_needed(&self, map: &mut HashMap<String, KeyRecord>, now_ms: u64) {
        if map.len() < self.max_capacity {
            return;
        }

        // 1. Evict entries whose penalty has expired and are idle for more than 1 hour.
        map.retain(|_, record| {
            if let Some(blocked) = record.blocked_until_ms
                && now_ms < blocked
            {
                return true;
            }
            now_ms.saturating_sub(record.last_accessed_ms) < 3_600_000
        });

        // 2. If still at capacity, evict the oldest accessed entries.
        if map.len() >= self.max_capacity {
            let mut items: Vec<(String, u64)> = map
                .iter()
                .map(|(key, rec)| (key.clone(), rec.last_accessed_ms))
                .collect();
            items.sort_unstable_by_key(|(_, ts)| *ts);
            let remove_count = (map.len() / 5).max(1);
            for (key, _) in items.into_iter().take(remove_count) {
                map.remove(&key);
            }
        }
    }
}

impl LimiterStore for MemoryLimiterStore {
    fn consume<'a>(
        &'a self,
        key: &'a str,
        amount: u64,
        limit: u64,
        window: Duration,
        algorithm: Algorithm,
    ) -> BoxFuture<'a, Result<RateLimitResult, LimiterError>> {
        Box::pin(async move {
            let now_ms = self.now_ms();
            let window_ms = window.as_millis().max(1) as u64;

            let mut map = self
                .entries
                .write()
                .map_err(|_| LimiterError::Store("Lock poisoning on write".to_string()))?;

            self.evict_if_needed(&mut map, now_ms);

            let record = map.entry(key.to_string()).or_insert_with(|| KeyRecord {
                data: match algorithm {
                    Algorithm::SlidingWindow => EntryData::SlidingWindow {
                        current_window: now_ms / window_ms,
                        current_count: 0,
                        previous_count: 0,
                    },
                    Algorithm::FixedWindow => EntryData::FixedWindow {
                        window_start: now_ms / window_ms,
                        count: 0,
                    },
                    Algorithm::TokenBucket => EntryData::TokenBucket {
                        tokens: limit as f64,
                        last_refill_ms: now_ms,
                    },
                },
                blocked_until_ms: None,
                last_accessed_ms: now_ms,
            });

            record.last_accessed_ms = now_ms;

            // Check if key is actively blocked
            if let Some(blocked_until) = record.blocked_until_ms {
                if now_ms < blocked_until {
                    let retry_after = Duration::from_millis(blocked_until - now_ms);
                    return Ok(RateLimitResult::new(
                        limit,
                        0,
                        retry_after,
                        false,
                        Some(retry_after),
                    ));
                }
                record.blocked_until_ms = None;
            }

            match (&mut record.data, algorithm) {
                (
                    EntryData::SlidingWindow {
                        current_window,
                        current_count,
                        previous_count,
                    },
                    Algorithm::SlidingWindow,
                ) => {
                    let this_window = now_ms / window_ms;
                    if *current_window == this_window {
                        // Same window
                    } else if *current_window + 1 == this_window {
                        *previous_count = *current_count;
                        *current_count = 0;
                        *current_window = this_window;
                    } else {
                        *previous_count = 0;
                        *current_count = 0;
                        *current_window = this_window;
                    }

                    let time_into_window_ms = now_ms % window_ms;
                    let weight = 1.0 - (time_into_window_ms as f64 / window_ms as f64);
                    let estimated_count = *current_count as f64 + (*previous_count as f64 * weight);
                    let reset_after_ms = window_ms - time_into_window_ms;
                    let reset_after = Duration::from_millis(reset_after_ms);

                    if estimated_count + amount as f64 <= limit as f64 {
                        *current_count = current_count.saturating_add(amount);
                        let remaining = limit.saturating_sub(*current_count);
                        Ok(RateLimitResult::new(
                            limit,
                            remaining,
                            reset_after,
                            true,
                            None,
                        ))
                    } else {
                        let remaining = 0;
                        Ok(RateLimitResult::new(
                            limit,
                            remaining,
                            reset_after,
                            false,
                            Some(reset_after),
                        ))
                    }
                }
                (
                    EntryData::FixedWindow {
                        window_start,
                        count,
                    },
                    Algorithm::FixedWindow,
                ) => {
                    let this_window = now_ms / window_ms;
                    if *window_start != this_window {
                        *window_start = this_window;
                        *count = 0;
                    }

                    let reset_after_ms = window_ms - (now_ms % window_ms);
                    let reset_after = Duration::from_millis(reset_after_ms);

                    if count.saturating_add(amount) <= limit {
                        *count = count.saturating_add(amount);
                        let remaining = limit.saturating_sub(*count);
                        Ok(RateLimitResult::new(
                            limit,
                            remaining,
                            reset_after,
                            true,
                            None,
                        ))
                    } else {
                        let remaining = limit.saturating_sub(*count);
                        Ok(RateLimitResult::new(
                            limit,
                            remaining,
                            reset_after,
                            false,
                            Some(reset_after),
                        ))
                    }
                }
                (
                    EntryData::TokenBucket {
                        tokens,
                        last_refill_ms,
                    },
                    Algorithm::TokenBucket,
                ) => {
                    let capacity = limit as f64;
                    let refill_rate = capacity / window.as_secs_f64().max(0.001);
                    let elapsed_secs = now_ms.saturating_sub(*last_refill_ms) as f64 / 1000.0;
                    *tokens = (*tokens + elapsed_secs * refill_rate).min(capacity);
                    *last_refill_ms = now_ms;

                    let reset_after = if *tokens < capacity {
                        let needed = capacity - *tokens;
                        Duration::from_secs_f64(needed / refill_rate)
                    } else {
                        Duration::from_millis(0)
                    };

                    if *tokens >= amount as f64 {
                        *tokens -= amount as f64;
                        let remaining = tokens.floor() as u64;
                        Ok(RateLimitResult::new(
                            limit,
                            remaining,
                            reset_after,
                            true,
                            None,
                        ))
                    } else {
                        let retry_after =
                            Duration::from_secs_f64((amount as f64 - *tokens) / refill_rate);
                        Ok(RateLimitResult::new(
                            limit,
                            tokens.floor() as u64,
                            reset_after,
                            false,
                            Some(retry_after),
                        ))
                    }
                }
                _ => {
                    // Reinitialize if algorithm changed for existing key
                    *record = KeyRecord {
                        data: match algorithm {
                            Algorithm::SlidingWindow => EntryData::SlidingWindow {
                                current_window: now_ms / window_ms,
                                current_count: amount,
                                previous_count: 0,
                            },
                            Algorithm::FixedWindow => EntryData::FixedWindow {
                                window_start: now_ms / window_ms,
                                count: amount,
                            },
                            Algorithm::TokenBucket => EntryData::TokenBucket {
                                tokens: (limit as f64 - amount as f64).max(0.0),
                                last_refill_ms: now_ms,
                            },
                        },
                        blocked_until_ms: None,
                        last_accessed_ms: now_ms,
                    };
                    let reset_after = Duration::from_millis(window_ms - (now_ms % window_ms));
                    let is_allowed = amount <= limit;
                    let remaining = limit.saturating_sub(amount);
                    let retry_after = if is_allowed { None } else { Some(reset_after) };
                    Ok(RateLimitResult::new(
                        limit,
                        remaining,
                        reset_after,
                        is_allowed,
                        retry_after,
                    ))
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
            let now_ms = self.now_ms();
            let window_ms = window.as_millis().max(1) as u64;

            let map = self
                .entries
                .read()
                .map_err(|_| LimiterError::Store("Lock poisoning on read".to_string()))?;

            let Some(record) = map.get(key) else {
                return Ok(RateLimitResult::new(limit, limit, window, true, None));
            };

            if let Some(blocked_until) = record.blocked_until_ms
                && now_ms < blocked_until
            {
                let retry_after = Duration::from_millis(blocked_until - now_ms);
                return Ok(RateLimitResult::new(
                    limit,
                    0,
                    retry_after,
                    false,
                    Some(retry_after),
                ));
            }

            match (&record.data, algorithm) {
                (
                    EntryData::SlidingWindow {
                        current_window,
                        current_count,
                        previous_count,
                    },
                    Algorithm::SlidingWindow,
                ) => {
                    let this_window = now_ms / window_ms;
                    let (curr, prev) = if *current_window == this_window {
                        (*current_count, *previous_count)
                    } else if *current_window + 1 == this_window {
                        (0, *current_count)
                    } else {
                        (0, 0)
                    };

                    let time_into_window_ms = now_ms % window_ms;
                    let weight = 1.0 - (time_into_window_ms as f64 / window_ms as f64);
                    let estimated_count = curr as f64 + (prev as f64 * weight);
                    let reset_after = Duration::from_millis(window_ms - time_into_window_ms);
                    let remaining = limit.saturating_sub(estimated_count.floor() as u64);
                    let is_allowed = remaining > 0;
                    let retry_after = if is_allowed { None } else { Some(reset_after) };

                    Ok(RateLimitResult::new(
                        limit,
                        remaining,
                        reset_after,
                        is_allowed,
                        retry_after,
                    ))
                }
                (
                    EntryData::FixedWindow {
                        window_start,
                        count,
                    },
                    Algorithm::FixedWindow,
                ) => {
                    let this_window = now_ms / window_ms;
                    let curr_count = if *window_start == this_window {
                        *count
                    } else {
                        0
                    };
                    let reset_after = Duration::from_millis(window_ms - (now_ms % window_ms));
                    let remaining = limit.saturating_sub(curr_count);
                    let is_allowed = remaining > 0;
                    let retry_after = if is_allowed { None } else { Some(reset_after) };

                    Ok(RateLimitResult::new(
                        limit,
                        remaining,
                        reset_after,
                        is_allowed,
                        retry_after,
                    ))
                }
                (
                    EntryData::TokenBucket {
                        tokens,
                        last_refill_ms,
                    },
                    Algorithm::TokenBucket,
                ) => {
                    let capacity = limit as f64;
                    let refill_rate = capacity / window.as_secs_f64().max(0.001);
                    let elapsed_secs = now_ms.saturating_sub(*last_refill_ms) as f64 / 1000.0;
                    let current_tokens = (*tokens + elapsed_secs * refill_rate).min(capacity);

                    let reset_after = if current_tokens < capacity {
                        Duration::from_secs_f64((capacity - current_tokens) / refill_rate)
                    } else {
                        Duration::from_millis(0)
                    };

                    let remaining = current_tokens.floor() as u64;
                    let is_allowed = remaining > 0;
                    let retry_after = if is_allowed { None } else { Some(reset_after) };

                    Ok(RateLimitResult::new(
                        limit,
                        remaining,
                        reset_after,
                        is_allowed,
                        retry_after,
                    ))
                }
                _ => Ok(RateLimitResult::new(limit, limit, window, true, None)),
            }
        })
    }

    fn reset<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), LimiterError>> {
        Box::pin(async move {
            let mut map = self
                .entries
                .write()
                .map_err(|_| LimiterError::Store("Lock poisoning on reset".to_string()))?;
            map.remove(key);
            Ok(())
        })
    }

    fn block<'a>(
        &'a self,
        key: &'a str,
        duration: Duration,
    ) -> BoxFuture<'a, Result<(), LimiterError>> {
        Box::pin(async move {
            let now_ms = self.now_ms();
            let block_until = now_ms.saturating_add(duration.as_millis() as u64);

            let mut map = self
                .entries
                .write()
                .map_err(|_| LimiterError::Store("Lock poisoning on block".to_string()))?;

            self.evict_if_needed(&mut map, now_ms);

            let record = map.entry(key.to_string()).or_insert_with(|| KeyRecord {
                data: EntryData::FixedWindow {
                    window_start: 0,
                    count: 0,
                },
                blocked_until_ms: Some(block_until),
                last_accessed_ms: now_ms,
            });

            record.blocked_until_ms = Some(block_until);
            record.last_accessed_ms = now_ms;
            Ok(())
        })
    }
}
