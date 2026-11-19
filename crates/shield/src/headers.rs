//! Security header policies, industry presets, and non-destructive header enforcement.

use std::collections::BTreeMap;

use router::{HeaderName, HeaderValue, Response};

use crate::constants::{
    CLEAR_SITE_DATA, CROSS_ORIGIN_EMBEDDER_POLICY, CROSS_ORIGIN_OPENER_POLICY,
    CROSS_ORIGIN_RESOURCE_POLICY, ORIGIN_AGENT_CLUSTER, PERMISSIONS_POLICY, REFERRER_POLICY,
    STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS, X_DOWNLOAD_OPTIONS, X_FRAME_OPTIONS,
    X_PERMITTED_CROSS_DOMAIN_POLICIES, X_XSS_PROTECTION,
};

/// Preset security profiles providing standard or hardened header suites.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SecurityPreset {
    /// Modern safe defaults compliant with general web standards.
    #[default]
    SecureDefaults,
    /// Strict lockdown conforming to OWASP recommended security headers.
    Owasp,
    /// Blank preset containing no default headers.
    Empty,
}

/// Action to enforce for the `X-Frame-Options` header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameAction {
    /// Deny rendering in frames or iframes entirely (`DENY`).
    Deny,
    /// Allow framing only by pages from the identical origin (`SAMEORIGIN`).
    SameOrigin,
}

impl FrameAction {
    /// Returns the HTTP header value string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Deny => "DENY",
            Self::SameOrigin => "SAMEORIGIN",
        }
    }
}

/// Directive values for the `Referrer-Policy` header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferrerPolicy {
    /// The `Referer` header will be omitted entirely.
    NoReferrer,
    /// Send full URL to HTTPS origins, but omit on protocol downgrade (e.g. HTTPS to HTTP).
    NoReferrerWhenDowngrade,
    /// Send only the origin (scheme, host, port) in the `Referer` header.
    Origin,
    /// Send origin for cross-origin requests, full URL for same-origin requests.
    OriginWhenCrossOrigin,
    /// Send full URL for same-origin requests only; omit for cross-origin requests.
    SameOrigin,
    /// Send origin only when security protocol level is preserved (HTTPS to HTTPS).
    StrictOrigin,
    /// Send full URL for same-origin, origin only for HTTPS cross-origin, omit on downgrade.
    StrictOriginWhenCrossOrigin,
    /// Send full URL for all requests (unsafe).
    UnsafeUrl,
}

impl ReferrerPolicy {
    /// Returns the HTTP header value string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoReferrer => "no-referrer",
            Self::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            Self::Origin => "origin",
            Self::OriginWhenCrossOrigin => "origin-when-cross-origin",
            Self::SameOrigin => "same-origin",
            Self::StrictOrigin => "strict-origin",
            Self::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            Self::UnsafeUrl => "unsafe-url",
        }
    }
}

/// Options for Adobe Flash and PDF cross-domain policy files (`X-Permitted-Cross-Domain-Policies`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossDomainPolicy {
    /// No policy files are permitted anywhere (`none`).
    None,
    /// Only master policy files are permitted (`master-only`).
    MasterOnly,
    /// Only policy files served with HTTP Content-Type text/x-cross-domain-policy are permitted.
    ByContentType,
    /// All policy files on this target domain are permitted (`all`).
    All,
}

impl CrossDomainPolicy {
    /// Returns the HTTP header value string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::MasterOnly => "master-only",
            Self::ByContentType => "by-content-type",
            Self::All => "all",
        }
    }
}

/// Configuration for HTTP Strict Transport Security (HSTS).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HstsConfig {
    /// Maximum time in seconds that the browser should remember this domain is only HTTPS.
    pub max_age: u64,
    /// Whether to apply HSTS to all subdomains.
    pub include_subdomains: bool,
    /// Whether to request inclusion in browser HSTS preload lists.
    pub preload: bool,
}

impl Default for HstsConfig {
    fn default() -> Self {
        Self {
            max_age: 31_536_000, // 1 year
            include_subdomains: true,
            preload: false,
        }
    }
}

impl HstsConfig {
    /// Formats the HSTS header value string.
    #[must_use]
    pub fn to_header_value(&self) -> String {
        let mut value = format!("max-age={}", self.max_age);
        if self.include_subdomains {
            value.push_str("; includeSubDomains");
        }
        if self.preload {
            value.push_str("; preload");
        }
        value
    }
}

/// Builder for constructing the W3C `Permissions-Policy` header.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PermissionsPolicyBuilder {
    features: BTreeMap<String, String>,
}

impl PermissionsPolicyBuilder {
    /// Creates a new empty `PermissionsPolicyBuilder`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicitly denies a feature for all origins: `feature=()`.
    #[must_use]
    pub fn deny(mut self, feature: impl Into<String>) -> Self {
        self.features.insert(feature.into(), "()".to_string());
        self
    }

    /// Allows a feature exclusively for the current origin: `feature=(self)`.
    #[must_use]
    pub fn allow_self(mut self, feature: impl Into<String>) -> Self {
        self.features.insert(feature.into(), "(self)".to_string());
        self
    }

    /// Allows a feature for all origins: `feature=*`.
    #[must_use]
    pub fn allow_all(mut self, feature: impl Into<String>) -> Self {
        self.features.insert(feature.into(), "*".to_string());
        self
    }

    /// Formats the complete `Permissions-Policy` header value.
    #[must_use]
    pub fn build(self) -> String {
        self.features
            .into_iter()
            .map(|(feature, allow)| format!("{feature}={allow}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Configuration for security response headers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityHeadersConfig {
    /// Active base security preset.
    pub preset: SecurityPreset,
    /// Action for `X-Frame-Options`.
    pub frame_guard: Option<FrameAction>,
    /// Configuration for `Strict-Transport-Security`.
    pub hsts: Option<HstsConfig>,
    /// Whether `X-Content-Type-Options: nosniff` is injected.
    pub no_sniff: bool,
    /// Optional custom value for `X-XSS-Protection`.
    pub xss_protection: Option<String>,
    /// Policy for `Referrer-Policy`.
    pub referrer_policy: Option<ReferrerPolicy>,
    /// Policy for `Permissions-Policy`.
    pub permissions_policy: Option<String>,
    /// Policy for `X-Permitted-Cross-Domain-Policies`.
    pub cross_domain_policy: Option<CrossDomainPolicy>,
    /// Optional value for `X-Download-Options`.
    pub download_options: Option<String>,
    /// Value for `Cross-Origin-Embedder-Policy`.
    pub coep: Option<String>,
    /// Value for `Cross-Origin-Opener-Policy`.
    pub coop: Option<String>,
    /// Value for `Cross-Origin-Resource-Policy`.
    pub corp: Option<String>,
    /// Whether `Origin-Agent-Cluster: ?1` is set.
    pub origin_agent_cluster: bool,
    /// Optional value for `Clear-Site-Data`.
    pub clear_site_data: Option<String>,
    /// Additional custom security headers.
    pub custom_headers: Vec<(String, String)>,
    /// When true, overwrites existing headers on the response. Default is false (non-destructive).
    pub overwrite_existing: bool,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self::from_preset(SecurityPreset::SecureDefaults)
    }
}

impl SecurityHeadersConfig {
    /// Creates a configuration from the specified preset.
    #[must_use]
    pub fn from_preset(preset: SecurityPreset) -> Self {
        match preset {
            SecurityPreset::SecureDefaults => Self {
                preset,
                frame_guard: Some(FrameAction::SameOrigin),
                hsts: Some(HstsConfig::default()),
                no_sniff: true,
                xss_protection: Some("0".to_string()),
                referrer_policy: Some(ReferrerPolicy::StrictOriginWhenCrossOrigin),
                permissions_policy: None,
                cross_domain_policy: None,
                download_options: None,
                coep: None,
                coop: None,
                corp: None,
                origin_agent_cluster: false,
                clear_site_data: None,
                custom_headers: Vec::new(),
                overwrite_existing: false,
            },
            SecurityPreset::Owasp => Self {
                preset,
                frame_guard: Some(FrameAction::Deny),
                hsts: Some(HstsConfig::default()),
                no_sniff: true,
                xss_protection: Some("0".to_string()),
                referrer_policy: Some(ReferrerPolicy::NoReferrer),
                permissions_policy: Some(
                    PermissionsPolicyBuilder::new()
                        .deny("accelerometer")
                        .deny("camera")
                        .deny("geolocation")
                        .deny("gyroscope")
                        .deny("magnetometer")
                        .deny("microphone")
                        .deny("payment")
                        .deny("usb")
                        .build(),
                ),
                cross_domain_policy: Some(CrossDomainPolicy::None),
                download_options: Some("noopen".to_string()),
                coep: Some("require-corp".to_string()),
                coop: Some("same-origin".to_string()),
                corp: Some("same-origin".to_string()),
                origin_agent_cluster: true,
                clear_site_data: None,
                custom_headers: Vec::new(),
                overwrite_existing: false,
            },
            SecurityPreset::Empty => Self {
                preset,
                frame_guard: None,
                hsts: None,
                no_sniff: false,
                xss_protection: None,
                referrer_policy: None,
                permissions_policy: None,
                cross_domain_policy: None,
                download_options: None,
                coep: None,
                coop: None,
                corp: None,
                origin_agent_cluster: false,
                clear_site_data: None,
                custom_headers: Vec::new(),
                overwrite_existing: false,
            },
        }
    }

    /// Creates a new builder initialized with secure defaults.
    #[must_use]
    pub fn builder() -> SecurityHeadersConfigBuilder {
        SecurityHeadersConfigBuilder::new()
    }

    /// Compiles all active security header pairs into a key-value list.
    #[must_use]
    pub fn compile_headers(&self) -> Vec<(String, String)> {
        let mut headers = Vec::new();

        if self.no_sniff {
            headers.push((X_CONTENT_TYPE_OPTIONS.to_string(), "nosniff".to_string()));
        }

        if let Some(frame) = self.frame_guard {
            headers.push((X_FRAME_OPTIONS.to_string(), frame.as_str().to_string()));
        }

        if let Some(hsts) = &self.hsts {
            headers.push((
                STRICT_TRANSPORT_SECURITY.to_string(),
                hsts.to_header_value(),
            ));
        }

        if let Some(xss) = &self.xss_protection {
            headers.push((X_XSS_PROTECTION.to_string(), xss.clone()));
        }

        if let Some(referrer) = self.referrer_policy {
            headers.push((REFERRER_POLICY.to_string(), referrer.as_str().to_string()));
        }

        if let Some(permissions) = &self.permissions_policy {
            headers.push((PERMISSIONS_POLICY.to_string(), permissions.clone()));
        }

        if let Some(cross_domain) = self.cross_domain_policy {
            headers.push((
                X_PERMITTED_CROSS_DOMAIN_POLICIES.to_string(),
                cross_domain.as_str().to_string(),
            ));
        }

        if let Some(download) = &self.download_options {
            headers.push((X_DOWNLOAD_OPTIONS.to_string(), download.clone()));
        }

        if let Some(coep) = &self.coep {
            headers.push((CROSS_ORIGIN_EMBEDDER_POLICY.to_string(), coep.clone()));
        }

        if let Some(coop) = &self.coop {
            headers.push((CROSS_ORIGIN_OPENER_POLICY.to_string(), coop.clone()));
        }

        if let Some(corp) = &self.corp {
            headers.push((CROSS_ORIGIN_RESOURCE_POLICY.to_string(), corp.clone()));
        }

        if self.origin_agent_cluster {
            headers.push((ORIGIN_AGENT_CLUSTER.to_string(), "?1".to_string()));
        }

        if let Some(clear_data) = &self.clear_site_data {
            headers.push((CLEAR_SITE_DATA.to_string(), clear_data.clone()));
        }

        for (key, val) in &self.custom_headers {
            headers.push((key.clone(), val.clone()));
        }

        headers
    }

    /// Applies configured security headers to the given response non-destructively.
    pub fn apply(&self, response: &mut Response) {
        let compiled = self.compile_headers();
        for (name_str, value_str) in compiled {
            if self.overwrite_existing || !response.headers().contains_key(name_str.as_str()) {
                response.set_header(HeaderName::new(name_str), HeaderValue::from(value_str));
            }
        }
    }
}

/// Fluent builder for [`SecurityHeadersConfig`].
#[derive(Clone, Debug, Default)]
pub struct SecurityHeadersConfigBuilder {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersConfigBuilder {
    /// Creates a new builder starting from `SecurityPreset::SecureDefaults`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: SecurityHeadersConfig::default(),
        }
    }

    /// Bases configuration on the given [`SecurityPreset`].
    #[must_use]
    pub fn preset(mut self, preset: SecurityPreset) -> Self {
        self.config = SecurityHeadersConfig::from_preset(preset);
        self
    }

    /// Configures the `X-Frame-Options` action.
    #[must_use]
    pub fn frame_guard(mut self, action: Option<FrameAction>) -> Self {
        self.config.frame_guard = action;
        self
    }

    /// Configures HTTP Strict Transport Security.
    #[must_use]
    pub fn hsts(mut self, hsts: Option<HstsConfig>) -> Self {
        self.config.hsts = hsts;
        self
    }

    /// Enables or disables `X-Content-Type-Options: nosniff`.
    #[must_use]
    pub fn no_sniff(mut self, enable: bool) -> Self {
        self.config.no_sniff = enable;
        self
    }

    /// Configures the `Referrer-Policy`.
    #[must_use]
    pub fn referrer_policy(mut self, policy: Option<ReferrerPolicy>) -> Self {
        self.config.referrer_policy = policy;
        self
    }

    /// Configures the `Permissions-Policy`.
    #[must_use]
    pub fn permissions_policy(mut self, policy: Option<String>) -> Self {
        self.config.permissions_policy = policy;
        self
    }

    /// Configures `Cross-Origin-Embedder-Policy`.
    #[must_use]
    pub fn coep(mut self, coep: Option<String>) -> Self {
        self.config.coep = coep;
        self
    }

    /// Configures `Cross-Origin-Opener-Policy`.
    #[must_use]
    pub fn coop(mut self, coop: Option<String>) -> Self {
        self.config.coop = coop;
        self
    }

    /// Configures `Cross-Origin-Resource-Policy`.
    #[must_use]
    pub fn corp(mut self, corp: Option<String>) -> Self {
        self.config.corp = corp;
        self
    }

    /// Configures `Origin-Agent-Cluster`.
    #[must_use]
    pub fn origin_agent_cluster(mut self, enable: bool) -> Self {
        self.config.origin_agent_cluster = enable;
        self
    }

    /// Appends a custom header name and value.
    #[must_use]
    pub fn custom_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.custom_headers.push((name.into(), value.into()));
        self
    }

    /// Configures whether to overwrite existing response headers.
    #[must_use]
    pub fn overwrite_existing(mut self, overwrite: bool) -> Self {
        self.config.overwrite_existing = overwrite;
        self
    }

    /// Finalizes and returns the configured [`SecurityHeadersConfig`].
    #[must_use]
    pub fn build(self) -> SecurityHeadersConfig {
        self.config
    }
}
