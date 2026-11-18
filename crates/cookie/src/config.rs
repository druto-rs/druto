//! Global and per-jar cookie attribute configuration policies.

use time::Duration;

use crate::same_site::SameSite;

/// Configuration policy determining default attributes applied to newly staged cookies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieConfig {
    /// Default `Path` attribute applied to cookies (default: `"/""`).
    pub default_path: String,
    /// Default `Domain` attribute applied to cookies (default: `None`).
    pub default_domain: Option<String>,
    /// Default `HttpOnly` flag (default: `true`).
    pub default_http_only: bool,
    /// Default `SameSite` policy (default: `Some(SameSite::Lax)`).
    pub default_same_site: Option<SameSite>,
    /// Default `Secure` flag (default: `false`, typically `true` in HTTPS production).
    pub default_secure: bool,
    /// Default `Partitioned` flag for CHIPS (default: `false`).
    pub default_partitioned: bool,
    /// Default `Max-Age` duration (default: `None`, session cookie).
    pub default_max_age: Option<Duration>,
}

impl Default for CookieConfig {
    fn default() -> Self {
        Self {
            default_path: "/".to_string(),
            default_domain: None,
            default_http_only: true,
            default_same_site: Some(SameSite::Lax),
            default_secure: false,
            default_partitioned: false,
            default_max_age: None,
        }
    }
}

impl CookieConfig {
    /// Creates a configuration with secure-by-default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default path.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.default_path = path.into();
        self
    }

    /// Sets the default domain.
    #[must_use]
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.default_domain = Some(domain.into());
        self
    }

    /// Sets the default HttpOnly flag.
    #[must_use]
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.default_http_only = http_only;
        self
    }

    /// Sets the default SameSite policy.
    #[must_use]
    pub fn same_site(mut self, same_site: impl Into<Option<SameSite>>) -> Self {
        self.default_same_site = same_site.into();
        self
    }

    /// Sets the default Secure flag.
    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.default_secure = secure;
        self
    }

    /// Sets the default Partitioned flag (CHIPS).
    #[must_use]
    pub fn partitioned(mut self, partitioned: bool) -> Self {
        self.default_partitioned = partitioned;
        self
    }

    /// Sets the default Max-Age duration.
    #[must_use]
    pub fn max_age(mut self, max_age: impl Into<Option<Duration>>) -> Self {
        self.default_max_age = max_age.into();
        self
    }
}
