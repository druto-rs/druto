//! Cross-Site Request Forgery (CSRF) defense, token lifecycle, and route exemptions.

use std::{fmt, sync::Arc};

use base64::prelude::*;
use cookie::{Cookie, SameSite};
use crypto::{hmac::HmacSigner, key::Key};
use router::{HttpContext, Response, StatusCode};
use session::SessionContext;
use subtle::ConstantTimeEq;

use crate::{
    constants::{
        DEFAULT_CSRF_FIELD_NAME, DEFAULT_CSRF_SESSION_KEY, DEFAULT_CSRF_TOKEN_FIELD_NAME,
        DEFAULT_XSRF_COOKIE_NAME, X_CSRF_TOKEN, X_XSRF_TOKEN,
    },
    error::ShieldError,
};

/// Type alias for dynamic CSRF route exemption predicate closures.
pub type CsrfPredicate = Arc<dyn Fn(&HttpContext) -> bool + Send + Sync>;

/// Strongly typed CSRF token issued for request verification and form helpers.
#[derive(Clone, PartialEq, Eq)]
pub struct CsrfToken(String);

impl CsrfToken {
    /// Creates a new `CsrfToken` wrapping the provided string.
    #[must_use]
    pub const fn new(token: String) -> Self {
        Self(token)
    }

    /// Returns the string slice representation of the CSRF token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the wrapper and returns the underlying `String`.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Debug for CsrfToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CsrfToken([PROTECTED])")
    }
}

impl fmt::Display for CsrfToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Defines the action taken when CSRF validation fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CsrfFailureMode {
    /// Responds immediately with HTTP 403 Forbidden without invoking downstream handlers.
    #[default]
    Response403,
    /// Aborts execution by returning a pipeline execution error.
    Error,
}

/// Configuration options for CSRF protection.
#[derive(Clone)]
pub struct CsrfConfig {
    /// Whether CSRF protection is active.
    pub enabled: bool,
    /// Session store key where the raw CSRF secret is persisted.
    pub session_key: String,
    /// Name of the cookie exposed to client JavaScript for AJAX requests.
    pub xsrf_cookie_name: String,
    /// Whether to issue the client-readable `XSRF-TOKEN` cookie.
    pub enable_xsrf_cookie: bool,
    /// Cookie path for the `XSRF-TOKEN` cookie.
    pub cookie_path: String,
    /// SameSite policy for the `XSRF-TOKEN` cookie.
    pub cookie_same_site: SameSite,
    /// Secure flag for the `XSRF-TOKEN` cookie.
    pub cookie_secure: bool,
    /// Failure action when CSRF verification fails.
    pub failure_mode: CsrfFailureMode,
    /// Path prefixes or glob patterns exempt from CSRF validation.
    pub except_routes: Vec<String>,
    /// Custom predicate closure to exempt requests dynamically.
    pub except_predicate: Option<CsrfPredicate>,
}

impl fmt::Debug for CsrfConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CsrfConfig")
            .field("enabled", &self.enabled)
            .field("session_key", &self.session_key)
            .field("xsrf_cookie_name", &self.xsrf_cookie_name)
            .field("enable_xsrf_cookie", &self.enable_xsrf_cookie)
            .field("cookie_path", &self.cookie_path)
            .field("cookie_same_site", &self.cookie_same_site)
            .field("cookie_secure", &self.cookie_secure)
            .field("failure_mode", &self.failure_mode)
            .field("except_routes", &self.except_routes)
            .field(
                "except_predicate",
                &self.except_predicate.as_ref().map(|_| "..."),
            )
            .finish()
    }
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            session_key: DEFAULT_CSRF_SESSION_KEY.to_string(),
            xsrf_cookie_name: DEFAULT_XSRF_COOKIE_NAME.to_string(),
            enable_xsrf_cookie: true,
            cookie_path: "/".to_string(),
            cookie_same_site: SameSite::Lax,
            cookie_secure: false,
            failure_mode: CsrfFailureMode::Response403,
            except_routes: Vec::new(),
            except_predicate: None,
        }
    }
}

impl CsrfConfig {
    /// Creates a new CSRF configuration builder.
    #[must_use]
    pub fn builder() -> CsrfConfigBuilder {
        CsrfConfigBuilder::new()
    }

    /// Determines whether the given request is exempt from CSRF validation.
    #[must_use]
    pub fn is_exempt(&self, ctx: &HttpContext) -> bool {
        let path = ctx.request.path();

        for route in &self.except_routes {
            if let Some(prefix) = route.strip_suffix("/*") {
                if path == prefix || path.starts_with(&format!("{prefix}/")) {
                    return true;
                }
            } else if let Some(prefix) = route.strip_suffix('*') {
                if path.starts_with(prefix) {
                    return true;
                }
            } else if path == route {
                return true;
            }
        }

        self.except_predicate
            .as_ref()
            .is_some_and(|predicate| predicate(ctx))
    }

    /// Resolves or initializes the CSRF secret key from the request context session or cookie jar.
    pub fn resolve_or_init_secret(&self, ctx: &mut HttpContext) -> Result<Key, ShieldError> {
        // 1. Check if an active session exists.
        if let Some(session) = ctx.session_mut() {
            let existing_key = session
                .get_value(&self.session_key)
                .and_then(serde_json::Value::as_str)
                .and_then(|val| Key::from_base64(val).ok());

            if let Some(key) = existing_key {
                return Ok(key);
            }

            // Fresh secret generation for the session.
            let fresh_key = Key::generate().map_err(|err| {
                ShieldError::InvalidConfiguration(format!("Failed to generate CSRF key: {err}"))
            })?;
            let serialized = BASE64_STANDARD.encode(fresh_key.as_slice());
            session.put_value(&self.session_key, serde_json::Value::String(serialized));

            return Ok(fresh_key);
        }

        // 2. Check if a secret was stored in request extensions.
        if let Some(existing_key) = ctx.extensions.get::<Key>() {
            return Ok(existing_key.clone());
        }

        // 3. Fallback: generate and cache key in request extensions.
        let fallback_key = Key::generate().map_err(|err| {
            ShieldError::InvalidConfiguration(format!(
                "Failed to generate fallback CSRF key: {err}"
            ))
        })?;
        ctx.extensions.insert(fallback_key.clone());
        Ok(fallback_key)
    }

    /// Generates a per-request salted/masked CSRF token.
    pub fn generate_token(&self, secret: &Key) -> Result<CsrfToken, ShieldError> {
        generate_csrf_token(secret)
    }

    /// Validates an incoming request token against the secret.
    pub fn verify_token(&self, secret: &Key, token: &str) -> bool {
        verify_csrf_token(secret, token)
    }

    /// Extracts the submitted CSRF token from headers, form payload, or query string.
    #[must_use]
    pub fn extract_token(&self, ctx: &HttpContext) -> Option<String> {
        extract_csrf_token(ctx)
    }

    /// Attaches the `XSRF-TOKEN` cookie to the context cookie jar.
    pub fn attach_xsrf_cookie(&self, ctx: &mut HttpContext, token: &CsrfToken) {
        if !self.enable_xsrf_cookie {
            return;
        }

        let cookie_result =
            Cookie::build(self.xsrf_cookie_name.clone(), token.as_str().to_string())
                .path(self.cookie_path.clone())
                .same_site(self.cookie_same_site)
                .secure(self.cookie_secure)
                .http_only(false)
                .build();

        if let Ok(cookie) = cookie_result {
            ctx.cookie_jar_mut().set(cookie);
            ctx.flush_cookies_to_response();
        }
    }

    /// Creates an immediate 403 Forbidden response for CSRF verification failure.
    #[must_use]
    pub fn forbidden_response(&self) -> Response {
        let mut response = Response::new(StatusCode::FORBIDDEN);
        response.text("403 Forbidden: Invalid or missing CSRF token");
        response
    }
}

/// Generates a 48-byte URL-safe Base64 token containing a 16-byte random salt and a 32-byte HMAC signature.
pub fn generate_csrf_token(secret: &Key) -> Result<CsrfToken, ShieldError> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|err| {
        ShieldError::InvalidConfiguration(format!("CSPRNG salt generation failed: {err}"))
    })?;

    let signer = HmacSigner::new(secret.clone());
    let signature = signer.sign(&salt);

    let mut token_bytes = [0u8; 48];
    token_bytes[0..16].copy_from_slice(&salt);
    token_bytes[16..48].copy_from_slice(&signature);

    let encoded = BASE64_URL_SAFE_NO_PAD.encode(token_bytes);
    Ok(CsrfToken::new(encoded))
}

/// Verifies a submitted CSRF token against the raw secret using constant-time comparison.
pub fn verify_csrf_token(secret: &Key, token: &str) -> bool {
    let decoded = match BASE64_URL_SAFE_NO_PAD.decode(token) {
        Ok(bytes) => bytes,
        Err(_) => match BASE64_STANDARD.decode(token) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        },
    };

    if decoded.len() != 48 {
        return false;
    }

    let salt = &decoded[0..16];
    let signature = &decoded[16..48];

    let signer = HmacSigner::new(secret.clone());
    let expected_sig = signer.sign(salt);

    expected_sig.ct_eq(signature).into()
}

/// Extracts a CSRF token from request headers, query string, or body payload.
#[must_use]
pub fn extract_csrf_token(ctx: &HttpContext) -> Option<String> {
    // 1. Check `X-CSRF-TOKEN` or `X-XSRF-TOKEN` request headers.
    for header in [X_CSRF_TOKEN, X_XSRF_TOKEN] {
        if let Some(val) = ctx.request.headers().get(header) {
            let token = val.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }

    // 2. Check query string parameters (`_csrf` or `_token`).
    if let Some(token) = ctx.request.query().and_then(find_token_in_url_encoded) {
        return Some(token);
    }

    // 3. Check body if available as UTF-8 text/form data.
    if let Ok(body_str) = std::str::from_utf8(ctx.request.body().as_ref()) {
        if let Some(token) = find_token_in_url_encoded(body_str) {
            return Some(token);
        }

        // Check simple JSON payload if body is formatted as JSON.
        if body_str.starts_with('{') {
            let parsed_opt = serde_json::from_str::<serde_json::Value>(body_str)
                .ok()
                .and_then(|json| {
                    json.get(DEFAULT_CSRF_FIELD_NAME)
                        .or_else(|| json.get(DEFAULT_CSRF_TOKEN_FIELD_NAME))
                        .and_then(serde_json::Value::as_str)
                        .filter(|s| !s.is_empty())
                        .map(ToString::to_string)
                });
            if let Some(token) = parsed_opt {
                return Some(token);
            }
        }
    }

    // 4. Check if previously stored in request extensions.
    if let Some(token) = ctx.extensions.get::<CsrfToken>() {
        return Some(token.as_str().to_string());
    }

    None
}

fn find_token_in_url_encoded(encoded: &str) -> Option<String> {
    for pair in encoded.split('&') {
        let (key, val) = pair.split_once('=').unwrap_or((pair, ""));
        if (key == DEFAULT_CSRF_FIELD_NAME || key == DEFAULT_CSRF_TOKEN_FIELD_NAME)
            && !val.is_empty()
        {
            return Some(val.to_string());
        }
    }
    None
}

/// Fluent builder for [`CsrfConfig`].
#[derive(Clone, Debug, Default)]
pub struct CsrfConfigBuilder {
    config: CsrfConfig,
}

impl CsrfConfigBuilder {
    /// Creates a new default CSRF configuration builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: CsrfConfig::default(),
        }
    }

    /// Enables or disables CSRF protection.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Configures the session key used to persist the raw secret.
    #[must_use]
    pub fn session_key(mut self, key: impl Into<String>) -> Self {
        self.config.session_key = key.into();
        self
    }

    /// Configures the client-accessible cookie name.
    #[must_use]
    pub fn xsrf_cookie_name(mut self, name: impl Into<String>) -> Self {
        self.config.xsrf_cookie_name = name.into();
        self
    }

    /// Enables or disables emission of the `XSRF-TOKEN` cookie.
    #[must_use]
    pub fn enable_xsrf_cookie(mut self, enable: bool) -> Self {
        self.config.enable_xsrf_cookie = enable;
        self
    }

    /// Sets the path for the `XSRF-TOKEN` cookie.
    #[must_use]
    pub fn cookie_path(mut self, path: impl Into<String>) -> Self {
        self.config.cookie_path = path.into();
        self
    }

    /// Sets the SameSite policy for the `XSRF-TOKEN` cookie.
    #[must_use]
    pub fn cookie_same_site(mut self, same_site: SameSite) -> Self {
        self.config.cookie_same_site = same_site;
        self
    }

    /// Sets the Secure flag for the `XSRF-TOKEN` cookie.
    #[must_use]
    pub fn cookie_secure(mut self, secure: bool) -> Self {
        self.config.cookie_secure = secure;
        self
    }

    /// Configures the failure mode when verification fails.
    #[must_use]
    pub fn failure_mode(mut self, mode: CsrfFailureMode) -> Self {
        self.config.failure_mode = mode;
        self
    }

    /// Adds a route path or pattern to exempt from CSRF validation.
    #[must_use]
    pub fn except_route(mut self, route: impl Into<String>) -> Self {
        self.config.except_routes.push(route.into());
        self
    }

    /// Adds multiple route paths or patterns to exempt from CSRF validation.
    #[must_use]
    pub fn except_routes<I, S>(mut self, routes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for route in routes {
            self.config.except_routes.push(route.into());
        }
        self
    }

    /// Sets a dynamic predicate to exempt requests from CSRF validation.
    #[must_use]
    pub fn except_predicate<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&HttpContext) -> bool + Send + Sync + 'static,
    {
        self.config.except_predicate = Some(Arc::new(predicate));
        self
    }

    /// Finalizes and returns the configured [`CsrfConfig`].
    #[must_use]
    pub fn build(self) -> CsrfConfig {
        self.config
    }
}
