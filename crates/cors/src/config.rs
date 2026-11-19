//! Cross-Origin Resource Sharing (CORS) configuration options, origin policies, and builders.

use std::{fmt, sync::Arc};

use regex::Regex;
use router::{HeaderName, HttpContext, Method, StatusCode};

use crate::constants::SIMPLE_EXPOSE_HEADERS;

/// Type alias for dynamic origin matching predicates.
pub type OriginPredicate = Arc<dyn Fn(&str, &HttpContext) -> bool + Send + Sync + 'static>;

/// Origin negotiation policy strategy.
#[derive(Clone)]
pub enum OriginPolicy {
    /// Allow any origin (`*`).
    ///
    /// When credentials are true, the request origin is reflected to comply
    /// with the W3C CORS specification.
    Any,
    /// Strict case-sensitive match against an explicit whitelist of origins.
    Exact(Vec<String>),
    /// Subdomain wildcard pattern matching (e.g. `https://*.example.com`).
    Pattern(Vec<String>),
    /// Always reflect the request's `Origin` header.
    Mirror,
    /// Runtime evaluation predicate inspecting request context.
    Dynamic(OriginPredicate),
}

impl OriginPolicy {
    /// Creates an `OriginPolicy` that allows any origin.
    #[must_use]
    pub fn any() -> Self {
        Self::Any
    }

    /// Creates an `OriginPolicy` matching an explicit whitelist of origin strings.
    #[must_use]
    pub fn exact(origins: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Exact(origins.into_iter().map(Into::into).collect())
    }

    /// Creates an `OriginPolicy` matching wildcard patterns (e.g. `https://*.example.com`).
    #[must_use]
    pub fn pattern(patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Pattern(patterns.into_iter().map(Into::into).collect())
    }

    /// Creates an `OriginPolicy` that mirrors the caller's origin.
    #[must_use]
    pub fn mirror() -> Self {
        Self::Mirror
    }

    /// Creates an `OriginPolicy` evaluating a dynamic predicate function against the origin and context.
    #[must_use]
    pub fn dynamic(predicate: impl Fn(&str, &HttpContext) -> bool + Send + Sync + 'static) -> Self {
        Self::Dynamic(Arc::new(predicate))
    }
}

impl fmt::Debug for OriginPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => formatter.write_str("OriginPolicy::Any"),
            Self::Exact(origins) => formatter
                .debug_tuple("OriginPolicy::Exact")
                .field(origins)
                .finish(),
            Self::Pattern(patterns) => formatter
                .debug_tuple("OriginPolicy::Pattern")
                .field(patterns)
                .finish(),
            Self::Mirror => formatter.write_str("OriginPolicy::Mirror"),
            Self::Dynamic(_) => formatter.write_str("OriginPolicy::Dynamic(<closure>)"),
        }
    }
}

/// Allowed headers negotiation policy strategy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderPolicy {
    /// Dynamically reflects requested headers from `Access-Control-Request-Headers`.
    Mirror,
    /// Explicit whitelist of allowed headers.
    List(Vec<HeaderName>),
}

impl HeaderPolicy {
    /// Creates a `HeaderPolicy` that echoes the requested headers.
    #[must_use]
    pub fn mirror() -> Self {
        Self::Mirror
    }

    /// Creates a `HeaderPolicy` restricting allowed headers to an explicit list.
    #[must_use]
    pub fn list(headers: impl IntoIterator<Item = impl Into<HeaderName>>) -> Self {
        Self::List(headers.into_iter().map(Into::into).collect())
    }
}

/// Allowed HTTP methods negotiation policy strategy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MethodPolicy {
    /// Allows all standard HTTP methods (`GET`, `HEAD`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS`).
    Any,
    /// Explicit whitelist of allowed HTTP methods.
    List(Vec<Method>),
}

impl MethodPolicy {
    /// Creates a `MethodPolicy` allowing all standard HTTP methods.
    #[must_use]
    pub fn any() -> Self {
        Self::Any
    }

    /// Creates a `MethodPolicy` restricting methods to an explicit list.
    #[must_use]
    pub fn list(methods: impl IntoIterator<Item = Method>) -> Self {
        Self::List(methods.into_iter().collect())
    }
}

/// Complete CORS middleware configuration.
#[derive(Clone, Debug)]
pub struct CorsConfig {
    /// Whether CORS processing is active.
    pub enabled: bool,
    /// URL path prefixes where CORS is enforced (e.g. `["api/*"]`).
    pub paths: Vec<String>,
    /// Origin negotiation strategy.
    pub origin: OriginPolicy,
    /// HTTP methods allowed for cross-origin requests.
    pub methods: MethodPolicy,
    /// Request headers allowed for cross-origin requests.
    pub headers: HeaderPolicy,
    /// Response headers exposed to client-side scripts.
    pub expose_headers: Vec<HeaderName>,
    /// Whether credentials (cookies, authorization headers, client certificates) are supported.
    pub credentials: bool,
    /// Caching duration in seconds for preflight responses (`Access-Control-Max-Age`).
    pub max_age: Option<u64>,
    /// HTTP status code returned on preflight responses.
    pub preflight_status: StatusCode,
    /// Pre-compiled regex patterns for wildcard origin matching.
    pub(crate) compiled_patterns: Arc<Vec<Regex>>,
    /// Pre-formatted comma-separated string for exposed headers excluding safelisted ones.
    pub(crate) normalized_expose_headers: Option<String>,
    /// Pre-formatted comma-separated string of allowed HTTP methods.
    pub(crate) formatted_methods: String,
}

impl CorsConfig {
    /// Creates a fluent `CorsConfigBuilder`.
    #[must_use]
    pub fn builder() -> CorsConfigBuilder {
        CorsConfigBuilder::new()
    }

    /// Checks whether the incoming request URI path matches the configured CORS path patterns.
    #[must_use]
    pub fn matches_request_path(&self, path: &str) -> bool {
        self.paths.iter().any(|pattern| matches_path(pattern, path))
    }

    /// Evaluates whether an origin is permitted by policy.
    #[must_use]
    pub fn is_origin_allowed(&self, origin: &str, ctx: &HttpContext) -> bool {
        match &self.origin {
            OriginPolicy::Any | OriginPolicy::Mirror => true,
            OriginPolicy::Exact(origins) => origins.iter().any(|allowed| allowed == origin),
            OriginPolicy::Pattern(_) => self
                .compiled_patterns
                .iter()
                .any(|pattern| pattern.is_match(origin)),
            OriginPolicy::Dynamic(predicate) => predicate(origin, ctx),
        }
    }

    /// Computes the value for the `Access-Control-Allow-Origin` response header.
    #[must_use]
    pub fn compute_allow_origin(&self, origin: &str, ctx: &HttpContext) -> Option<String> {
        if !self.is_origin_allowed(origin, ctx) {
            return None;
        }

        match &self.origin {
            OriginPolicy::Any => {
                if self.credentials {
                    // W3C CORS forbids literal '*' when credentials are true.
                    Some(origin.to_string())
                } else {
                    Some("*".to_string())
                }
            }
            OriginPolicy::Mirror
            | OriginPolicy::Exact(_)
            | OriginPolicy::Pattern(_)
            | OriginPolicy::Dynamic(_) => Some(origin.to_string()),
        }
    }

    /// Validates whether a requested HTTP method is permitted.
    #[must_use]
    pub fn is_method_allowed(&self, method: &Method) -> bool {
        match &self.methods {
            MethodPolicy::Any => matches!(
                method,
                Method::Get
                    | Method::Head
                    | Method::Post
                    | Method::Put
                    | Method::Patch
                    | Method::Delete
                    | Method::Options
            ),
            MethodPolicy::List(allowed) => allowed.contains(method),
        }
    }

    /// Validates whether all requested header names are permitted.
    #[must_use]
    pub fn are_headers_allowed(&self, requested_headers: &[&str]) -> bool {
        match &self.headers {
            HeaderPolicy::Mirror => true,
            HeaderPolicy::List(allowed) => {
                let allowed_set: std::collections::HashSet<String> = allowed
                    .iter()
                    .map(|header| header.as_str().to_ascii_lowercase())
                    .collect();

                requested_headers.iter().all(|req_header| {
                    let trimmed = req_header.trim();
                    // Origin header is always permitted per CORS specification.
                    trimmed.eq_ignore_ascii_case("origin")
                        || allowed_set.contains(&trimmed.to_ascii_lowercase())
                })
            }
        }
    }

    /// Returns the pre-formatted string of allowed HTTP methods.
    #[must_use]
    pub fn formatted_methods(&self) -> &str {
        &self.formatted_methods
    }

    /// Returns the normalized string of exposed headers, if any.
    #[must_use]
    pub fn normalized_expose_headers(&self) -> Option<&str> {
        self.normalized_expose_headers.as_deref()
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        CorsConfigBuilder::new().build()
    }
}

/// Fluent builder for constructing [`CorsConfig`].
#[derive(Clone, Debug)]
pub struct CorsConfigBuilder {
    enabled: bool,
    paths: Vec<String>,
    origin: OriginPolicy,
    methods: MethodPolicy,
    headers: HeaderPolicy,
    expose_headers: Vec<HeaderName>,
    credentials: bool,
    max_age: Option<u64>,
    preflight_status: StatusCode,
}

impl CorsConfigBuilder {
    /// Creates a new builder initialized with default configuration values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: true,
            paths: vec!["*".to_string()],
            origin: OriginPolicy::Any,
            methods: MethodPolicy::Any,
            headers: HeaderPolicy::Mirror,
            expose_headers: Vec::new(),
            credentials: false,
            max_age: Some(90),
            preflight_status: StatusCode::NO_CONTENT,
        }
    }

    /// Sets whether the CORS middleware is enabled.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the list of path prefixes where CORS is enforced.
    #[must_use]
    pub fn paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.paths = paths.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a single path prefix where CORS is enforced.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Sets the origin policy.
    #[must_use]
    pub fn allow_origin(mut self, policy: OriginPolicy) -> Self {
        self.origin = policy;
        self
    }

    /// Configures the origin policy to allow any origin.
    #[must_use]
    pub fn allow_any_origin(self) -> Self {
        self.allow_origin(OriginPolicy::Any)
    }

    /// Configures the origin policy with an explicit whitelist of origins.
    #[must_use]
    pub fn allow_origins(self, origins: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allow_origin(OriginPolicy::exact(origins))
    }

    /// Configures the origin policy with subdomain wildcard patterns.
    #[must_use]
    pub fn allow_origin_patterns(
        self,
        patterns: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.allow_origin(OriginPolicy::pattern(patterns))
    }

    /// Configures the origin policy to mirror the incoming request origin.
    #[must_use]
    pub fn mirror_origin(self) -> Self {
        self.allow_origin(OriginPolicy::Mirror)
    }

    /// Configures the origin policy with a dynamic evaluation predicate.
    #[must_use]
    pub fn dynamic_origin(
        self,
        predicate: impl Fn(&str, &HttpContext) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.allow_origin(OriginPolicy::dynamic(predicate))
    }

    /// Sets the allowed HTTP methods policy.
    #[must_use]
    pub fn allow_methods(mut self, policy: MethodPolicy) -> Self {
        self.methods = policy;
        self
    }

    /// Configures methods policy to allow all standard HTTP methods.
    #[must_use]
    pub fn allow_any_methods(self) -> Self {
        self.allow_methods(MethodPolicy::Any)
    }

    /// Configures methods policy with an explicit list of allowed HTTP methods.
    #[must_use]
    pub fn allow_method_list(self, methods: impl IntoIterator<Item = Method>) -> Self {
        self.allow_methods(MethodPolicy::list(methods))
    }

    /// Sets the allowed request headers policy.
    #[must_use]
    pub fn allow_headers(mut self, policy: HeaderPolicy) -> Self {
        self.headers = policy;
        self
    }

    /// Configures headers policy to dynamically mirror requested headers.
    #[must_use]
    pub fn mirror_headers(self) -> Self {
        self.allow_headers(HeaderPolicy::Mirror)
    }

    /// Configures headers policy with an explicit whitelist of allowed header names.
    #[must_use]
    pub fn allow_header_list(
        self,
        headers: impl IntoIterator<Item = impl Into<HeaderName>>,
    ) -> Self {
        self.allow_headers(HeaderPolicy::list(headers))
    }

    /// Sets the list of response headers exposed to browser scripts.
    #[must_use]
    pub fn expose_headers(
        mut self,
        headers: impl IntoIterator<Item = impl Into<HeaderName>>,
    ) -> Self {
        self.expose_headers = headers.into_iter().map(Into::into).collect();
        self
    }

    /// Adds a single response header to the exposed headers list.
    #[must_use]
    pub fn expose_header(mut self, header: impl Into<HeaderName>) -> Self {
        self.expose_headers.push(header.into());
        self
    }

    /// Sets whether credentials (cookies, authorization headers, client certs) are permitted.
    #[must_use]
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.credentials = allow;
        self
    }

    /// Sets the caching duration in seconds for preflight responses.
    #[must_use]
    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Disables preflight response caching.
    #[must_use]
    pub fn no_max_age(mut self) -> Self {
        self.max_age = None;
        self
    }

    /// Sets the HTTP status code returned on preflight responses.
    #[must_use]
    pub fn preflight_status(mut self, status: StatusCode) -> Self {
        self.preflight_status = status;
        self
    }

    /// Builds and returns the finalized [`CorsConfig`].
    #[must_use]
    pub fn build(self) -> CorsConfig {
        let compiled_patterns = match &self.origin {
            OriginPolicy::Pattern(patterns) => {
                let mut regexes = Vec::with_capacity(patterns.len());
                for pat in patterns {
                    let escaped = regex::escape(pat).replace(r"\*", ".*");
                    if let Ok(re) = Regex::new(&format!("^{escaped}$")) {
                        regexes.push(re);
                    }
                }
                Arc::new(regexes)
            }
            _ => Arc::new(Vec::new()),
        };

        let normalized_expose_headers = normalize_expose_headers(&self.expose_headers);
        let formatted_methods = format_methods(&self.methods);

        CorsConfig {
            enabled: self.enabled,
            paths: self.paths,
            origin: self.origin,
            methods: self.methods,
            headers: self.headers,
            expose_headers: self.expose_headers,
            credentials: self.credentials,
            max_age: self.max_age,
            preflight_status: self.preflight_status,
            compiled_patterns,
            normalized_expose_headers,
            formatted_methods,
        }
    }
}

impl Default for CorsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Matches a URI path against a wildcard or prefix pattern.
#[must_use]
pub fn matches_path(pattern: &str, path: &str) -> bool {
    if pattern == "*" || pattern == "/*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return path.starts_with(prefix)
            && (path.len() == prefix.len() || path.as_bytes().get(prefix.len()) == Some(&b'/'));
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    pattern == path
}

fn normalize_expose_headers(headers: &[HeaderName]) -> Option<String> {
    let non_safelisted: Vec<&str> = headers
        .iter()
        .map(HeaderName::as_str)
        .filter(|header| !SIMPLE_EXPOSE_HEADERS.contains(&header.to_ascii_lowercase().as_str()))
        .collect();

    if non_safelisted.is_empty() {
        None
    } else {
        Some(non_safelisted.join(","))
    }
}

fn format_methods(policy: &MethodPolicy) -> String {
    match policy {
        MethodPolicy::Any => "GET,HEAD,POST,PUT,PATCH,DELETE,OPTIONS".to_string(),
        MethodPolicy::List(methods) => methods
            .iter()
            .map(Method::as_str)
            .collect::<Vec<_>>()
            .join(","),
    }
}
