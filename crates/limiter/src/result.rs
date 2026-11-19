//! Snapshot models and evaluation outputs for rate limiting operations.

use std::time::Duration;

/// Evaluation outcome from a rate limiter inspection or consumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitResult {
    /// Maximum allowed units within the configured window.
    pub limit: u64,
    /// Remaining quota units available in the current window.
    pub remaining: u64,
    /// Duration until the current window resets.
    pub reset_after: Duration,
    /// Whether the evaluated request was permitted under the policy.
    pub is_allowed: bool,
    /// Suggested wait duration before retrying if quota is exceeded.
    pub retry_after: Option<Duration>,
}

impl RateLimitResult {
    /// Creates a new `RateLimitResult`.
    #[must_use]
    pub fn new(
        limit: u64,
        remaining: u64,
        reset_after: Duration,
        is_allowed: bool,
        retry_after: Option<Duration>,
    ) -> Self {
        Self {
            limit,
            remaining,
            reset_after,
            is_allowed,
            retry_after,
        }
    }

    /// Returns `true` if the request was allowed through.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        self.is_allowed
    }

    /// Returns `true` if the quota has been fully exhausted.
    #[must_use]
    pub fn is_exceeded(&self) -> bool {
        !self.is_allowed
    }

    /// Returns reset duration rounded up to whole seconds.
    #[must_use]
    pub fn reset_seconds(&self) -> u64 {
        let secs = self.reset_after.as_secs();
        if self.reset_after.subsec_nanos() > 0 {
            secs.saturating_add(1)
        } else {
            secs.max(1)
        }
    }

    /// Returns retry duration in delta seconds if rate limit is exceeded.
    #[must_use]
    pub fn retry_after_seconds(&self) -> u64 {
        match self.retry_after {
            Some(duration) => {
                let secs = duration.as_secs();
                if duration.subsec_nanos() > 0 {
                    secs.saturating_add(1)
                } else {
                    secs.max(1)
                }
            }
            None => 0,
        }
    }

    /// Merges two evaluation results, picking the most restrictive constraints.
    #[must_use]
    pub fn combine_most_restrictive(&self, other: &Self) -> Self {
        let is_allowed = self.is_allowed && other.is_allowed;
        let limit = self.limit.min(other.limit);
        let remaining = self.remaining.min(other.remaining);
        let reset_after = self.reset_after.max(other.reset_after);
        let retry_after = match (self.retry_after, other.retry_after) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        Self {
            limit,
            remaining,
            reset_after,
            is_allowed,
            retry_after,
        }
    }
}
