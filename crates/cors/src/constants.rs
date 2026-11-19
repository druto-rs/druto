//! Standard HTTP header constants and safelisted header collections for CORS.

use router::HeaderName;

/// `Origin` header name.
pub const ORIGIN: HeaderName = HeaderName::from_static("origin");

/// `Access-Control-Request-Method` header name.
pub const ACCESS_CONTROL_REQUEST_METHOD: HeaderName =
    HeaderName::from_static("access-control-request-method");

/// `Access-Control-Request-Headers` header name.
pub const ACCESS_CONTROL_REQUEST_HEADERS: HeaderName =
    HeaderName::from_static("access-control-request-headers");

/// `Access-Control-Allow-Origin` header name.
pub const ACCESS_CONTROL_ALLOW_ORIGIN: HeaderName =
    HeaderName::from_static("access-control-allow-origin");

/// `Access-Control-Allow-Credentials` header name.
pub const ACCESS_CONTROL_ALLOW_CREDENTIALS: HeaderName =
    HeaderName::from_static("access-control-allow-credentials");

/// `Access-Control-Allow-Methods` header name.
pub const ACCESS_CONTROL_ALLOW_METHODS: HeaderName =
    HeaderName::from_static("access-control-allow-methods");

/// `Access-Control-Allow-Headers` header name.
pub const ACCESS_CONTROL_ALLOW_HEADERS: HeaderName =
    HeaderName::from_static("access-control-allow-headers");

/// `Access-Control-Expose-Headers` header name.
pub const ACCESS_CONTROL_EXPOSE_HEADERS: HeaderName =
    HeaderName::from_static("access-control-expose-headers");

/// `Access-Control-Max-Age` header name.
pub const ACCESS_CONTROL_MAX_AGE: HeaderName = HeaderName::from_static("access-control-max-age");

/// `Vary` header name.
pub const VARY: HeaderName = HeaderName::VARY;

/// Standard CORS-safelisted response header names that browsers expose to client scripts by default.
pub const SIMPLE_EXPOSE_HEADERS: &[&str] = &[
    "cache-control",
    "content-language",
    "content-type",
    "expires",
    "last-modified",
    "pragma",
];
