//! Canonical HTTP header constants and default values for security hardening.

/// Header for Content Security Policy.
pub const CONTENT_SECURITY_POLICY: &str = "content-security-policy";

/// Header for Content Security Policy Report-Only mode.
pub const CONTENT_SECURITY_POLICY_REPORT_ONLY: &str = "content-security-policy-report-only";

/// Header for HTTP Strict Transport Security.
pub const STRICT_TRANSPORT_SECURITY: &str = "strict-transport-security";

/// Header for MIME type sniffing prevention.
pub const X_CONTENT_TYPE_OPTIONS: &str = "x-content-type-options";

/// Header for clickjacking protection.
pub const X_FRAME_OPTIONS: &str = "x-frame-options";

/// Header for Referrer-Policy controls.
pub const REFERRER_POLICY: &str = "referrer-policy";

/// Header for browser feature and hardware delegation permissions.
pub const PERMISSIONS_POLICY: &str = "permissions-policy";

/// Legacy header to disable browser reflected XSS auditing.
pub const X_XSS_PROTECTION: &str = "x-xss-protection";

/// Header to instruct Internet Explorer not to execute downloads in application context.
pub const X_DOWNLOAD_OPTIONS: &str = "x-download-options";

/// Header to restrict Adobe Flash and PDF cross-domain policy files.
pub const X_PERMITTED_CROSS_DOMAIN_POLICIES: &str = "x-permitted-cross-domain-policies";

/// Header for Cross-Origin Embedder Policy.
pub const CROSS_ORIGIN_EMBEDDER_POLICY: &str = "cross-origin-embedder-policy";

/// Header for Cross-Origin Opener Policy.
pub const CROSS_ORIGIN_OPENER_POLICY: &str = "cross-origin-opener-policy";

/// Header for Cross-Origin Resource Policy.
pub const CROSS_ORIGIN_RESOURCE_POLICY: &str = "cross-origin-resource-policy";

/// Header to hint browser origin agent clustering.
pub const ORIGIN_AGENT_CLUSTER: &str = "origin-agent-cluster";

/// Header to instruct browser to clear cache, cookies, or storage.
pub const CLEAR_SITE_DATA: &str = "clear-site-data";

/// Header for Cache-Control directives.
pub const CACHE_CONTROL: &str = "cache-control";

/// Header name commonly used for transmitting CSRF tokens via AJAX or fetch.
pub const X_CSRF_TOKEN: &str = "x-csrf-token";

/// Alternative header name used for transmitting CSRF tokens in single-page applications.
pub const X_XSRF_TOKEN: &str = "x-xsrf-token";

/// Default cookie name for client-accessible CSRF tokens.
pub const DEFAULT_XSRF_COOKIE_NAME: &str = "XSRF-TOKEN";

/// Default session key used to persist the raw CSRF secret.
pub const DEFAULT_CSRF_SESSION_KEY: &str = "druto.csrf.secret";

/// Form field or query parameter key inspected for CSRF tokens.
pub const DEFAULT_CSRF_FIELD_NAME: &str = "_csrf";

/// Alternative form field or query parameter key inspected for CSRF tokens.
pub const DEFAULT_CSRF_TOKEN_FIELD_NAME: &str = "_token";
