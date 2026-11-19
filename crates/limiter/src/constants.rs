//! Standardized and legacy HTTP rate limiting header name constants.

use router::HeaderName;

/// `RateLimit-Limit` (IETF draft)
pub const RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("ratelimit-limit");
/// `RateLimit-Remaining` (IETF draft)
pub const RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("ratelimit-remaining");
/// `RateLimit-Reset` (IETF draft)
pub const RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("ratelimit-reset");
/// `RateLimit-Policy` (IETF draft)
pub const RATE_LIMIT_POLICY: HeaderName = HeaderName::from_static("ratelimit-policy");

/// `X-RateLimit-Limit` (Legacy)
pub const X_RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
/// `X-RateLimit-Remaining` (Legacy)
pub const X_RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
/// `X-RateLimit-Reset` (Legacy)
pub const X_RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");

/// `Retry-After` (RFC 7231 / RFC 9110)
pub const RETRY_AFTER: HeaderName = HeaderName::from_static("retry-after");
