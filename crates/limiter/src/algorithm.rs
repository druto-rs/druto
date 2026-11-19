//! Rate limiting algorithm definitions and mathematical evaluation models.

use std::fmt;

/// Supported rate limiting algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Algorithm {
    /// Sliding window counter approximating rolling windows with weighted past window overlap.
    #[default]
    SlidingWindow,
    /// Fixed window counter resetting on discrete modulo intervals.
    FixedWindow,
    /// Token bucket permitting bursts up to capacity while refilling continuously.
    TokenBucket,
}

impl fmt::Display for Algorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SlidingWindow => formatter.write_str("sliding_window"),
            Self::FixedWindow => formatter.write_str("fixed_window"),
            Self::TokenBucket => formatter.write_str("token_bucket"),
        }
    }
}
