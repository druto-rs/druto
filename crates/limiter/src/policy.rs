//! Rate limiting quota policies and multi-tier configuration definitions.

use std::time::Duration;

use crate::algorithm::Algorithm;

/// Configuration parameters for a single rate limit tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierConfig {
    /// Maximum allowed request count within the window.
    pub limit: u64,
    /// Duration of the rate limit window.
    pub window: Duration,
    /// Algorithm to apply for this tier.
    pub algorithm: Algorithm,
    /// Optional lockout penalty duration applied when quota is exhausted.
    pub block_for: Option<Duration>,
}

impl TierConfig {
    /// Constructs a new `TierConfig`.
    #[must_use]
    pub fn new(limit: u64, window: Duration) -> Self {
        Self {
            limit,
            window,
            algorithm: Algorithm::SlidingWindow,
            block_for: None,
        }
    }

    /// Sets the algorithm for this tier.
    #[must_use]
    pub fn algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Sets the penalty duration applied upon quota exhaustion.
    #[must_use]
    pub fn block_for(mut self, duration: Duration) -> Self {
        self.block_for = Some(duration);
        self
    }
}

/// Dynamic quota policy evaluated per request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaPolicy {
    /// One or more tiered limits to be evaluated for the request.
    Limit(Vec<TierConfig>),
    /// Bypasses rate limiting entirely.
    Unlimited,
}

impl QuotaPolicy {
    /// Creates an unlimited quota policy bypassing all checks.
    #[must_use]
    pub fn unlimited() -> Self {
        Self::Unlimited
    }

    /// Creates a policy with a custom limit and window duration.
    #[must_use]
    pub fn tier(limit: u64, window: Duration) -> Self {
        Self::Limit(vec![TierConfig::new(limit, window)])
    }

    /// Creates a policy permitting `limit` requests per second.
    #[must_use]
    pub fn per_second(limit: u64) -> Self {
        Self::tier(limit, Duration::from_secs(1))
    }

    /// Creates a policy permitting `limit` requests per minute.
    #[must_use]
    pub fn per_minute(limit: u64) -> Self {
        Self::tier(limit, Duration::from_secs(60))
    }

    /// Creates a policy permitting `limit` requests per hour.
    #[must_use]
    pub fn per_hour(limit: u64) -> Self {
        Self::tier(limit, Duration::from_secs(3600))
    }

    /// Creates a policy permitting `limit` requests per day.
    #[must_use]
    pub fn per_day(limit: u64) -> Self {
        Self::tier(limit, Duration::from_secs(86400))
    }

    /// Chains an additional limit tier to this policy.
    #[must_use]
    pub fn and(mut self, limit: u64, window: Duration) -> Self {
        match &mut self {
            Self::Limit(tiers) => {
                tiers.push(TierConfig::new(limit, window));
                self
            }
            Self::Unlimited => Self::Limit(vec![TierConfig::new(limit, window)]),
        }
    }

    /// Sets the algorithm on the most recently added tier.
    #[must_use]
    pub fn algorithm(mut self, algorithm: Algorithm) -> Self {
        if let Self::Limit(tiers) = &mut self
            && let Some(last) = tiers.last_mut()
        {
            last.algorithm = algorithm;
        }
        self
    }

    /// Sets the penalty lockout duration on the most recently added tier.
    #[must_use]
    pub fn block_for(mut self, duration: Duration) -> Self {
        if let Self::Limit(tiers) = &mut self
            && let Some(last) = tiers.last_mut()
        {
            last.block_for = Some(duration);
        }
        self
    }

    /// Returns `true` if this policy is unlimited.
    #[must_use]
    pub fn is_unlimited(&self) -> bool {
        matches!(self, Self::Unlimited)
    }

    /// Returns a slice of the configured limit tiers.
    #[must_use]
    pub fn tiers(&self) -> &[TierConfig] {
        match self {
            Self::Limit(tiers) => tiers.as_slice(),
            Self::Unlimited => &[],
        }
    }
}
