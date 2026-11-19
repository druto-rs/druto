//! W3C Content Security Policy (CSP) builder, report-only mode, and dynamic per-request nonces.

use std::collections::BTreeMap;

use base64::prelude::*;
use router::{HeaderName, HeaderValue, Response};

use crate::{
    constants::{CONTENT_SECURITY_POLICY, CONTENT_SECURITY_POLICY_REPORT_ONLY},
    error::ShieldError,
};

/// Strongly typed cryptographic 128-bit CSPRNG nonce stored in request extensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CspNonce(String);

impl CspNonce {
    /// Creates a new `CspNonce` wrapping the provided raw string.
    #[must_use]
    pub const fn new(nonce: String) -> Self {
        Self(nonce)
    }

    /// Generates a fresh 16-byte (128-bit) CSPRNG nonce encoded as standard Base64.
    pub fn generate() -> Result<Self, ShieldError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|err| {
            ShieldError::InvalidConfiguration(format!("CSPRNG nonce generation failed: {err}"))
        })?;
        Ok(Self(BASE64_STANDARD.encode(bytes)))
    }

    /// Returns the string slice representation of the nonce.
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

/// Content Security Policy (CSP) configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CspConfig {
    /// Whether CSP header injection is active.
    pub enabled: bool,
    /// Ordered map of directive names to source lists.
    pub directives: BTreeMap<String, Vec<String>>,
    /// When true, emits `Content-Security-Policy-Report-Only` instead of enforcing.
    pub report_only: bool,
    /// Whether to automatically generate and attach a per-request [`CspNonce`].
    pub generate_nonce: bool,
    /// When true, overwrites any pre-existing CSP header on the response. Default is false.
    pub overwrite_existing: bool,
}

impl Default for CspConfig {
    fn default() -> Self {
        let mut directives = BTreeMap::new();
        directives.insert("default-src".to_string(), vec!["'self'".to_string()]);
        directives.insert("base-uri".to_string(), vec!["'self'".to_string()]);
        directives.insert("object-src".to_string(), vec!["'none'".to_string()]);

        Self {
            enabled: true,
            directives,
            report_only: false,
            generate_nonce: false,
            overwrite_existing: false,
        }
    }
}

impl CspConfig {
    /// Creates a new CSP builder with empty directives.
    #[must_use]
    pub fn builder() -> CspConfigBuilder {
        CspConfigBuilder::new()
    }

    /// Returns the canonical HTTP header name depending on report-only mode.
    #[must_use]
    pub const fn header_name(&self) -> &'static str {
        if self.report_only {
            CONTENT_SECURITY_POLICY_REPORT_ONLY
        } else {
            CONTENT_SECURITY_POLICY
        }
    }

    /// Formats the compiled CSP header string, substituting `@nonce` placeholders with the active nonce.
    #[must_use]
    pub fn format_header(&self, nonce: Option<&CspNonce>) -> String {
        let mut parts = Vec::with_capacity(self.directives.len());

        for (directive, sources) in &self.directives {
            if sources.is_empty() {
                parts.push(directive.clone());
                continue;
            }

            let resolved_sources: Vec<String> = sources
                .iter()
                .map(|source| {
                    if source == "@nonce" {
                        if let Some(active_nonce) = nonce {
                            format!("'nonce-{}'", active_nonce.as_str())
                        } else {
                            // Fallback if no nonce is active.
                            "'self'".to_string()
                        }
                    } else {
                        source.clone()
                    }
                })
                .collect();

            parts.push(format!("{directive} {}", resolved_sources.join(" ")));
        }

        parts.join("; ")
    }

    /// Applies the CSP header to the outgoing response non-destructively.
    pub fn apply(&self, response: &mut Response, nonce: Option<&CspNonce>) {
        if !self.enabled || self.directives.is_empty() {
            return;
        }

        if self.overwrite_existing || !response.headers().contains_key(self.header_name()) {
            let value = self.format_header(nonce);
            response.set_header(
                HeaderName::new(self.header_name()),
                HeaderValue::from(value),
            );
        }
    }
}

/// Fluent builder for [`CspConfig`].
#[derive(Clone, Debug, Default)]
pub struct CspConfigBuilder {
    enabled: bool,
    directives: BTreeMap<String, Vec<String>>,
    report_only: bool,
    generate_nonce: bool,
    overwrite_existing: bool,
}

impl CspConfigBuilder {
    /// Creates an empty CSP builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            enabled: true,
            directives: BTreeMap::new(),
            report_only: false,
            generate_nonce: false,
            overwrite_existing: false,
        }
    }

    /// Enables or disables CSP header emission.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets report-only mode (`Content-Security-Policy-Report-Only`).
    #[must_use]
    pub fn report_only(mut self, report_only: bool) -> Self {
        self.report_only = report_only;
        self
    }

    /// Enables automatic per-request CSP nonce generation.
    #[must_use]
    pub fn generate_nonce(mut self, generate: bool) -> Self {
        self.generate_nonce = generate;
        self
    }

    /// Configures whether to overwrite existing response headers.
    #[must_use]
    pub fn overwrite_existing(mut self, overwrite: bool) -> Self {
        self.overwrite_existing = overwrite;
        self
    }

    /// Adds or replaces a directive with the specified sources.
    #[must_use]
    pub fn directive<I, S>(mut self, name: impl Into<String>, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let list: Vec<String> = sources.into_iter().map(Into::into).collect();
        self.directives.insert(name.into(), list);
        self
    }

    /// Configures `default-src`.
    #[must_use]
    pub fn default_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("default-src", sources)
    }

    /// Configures `script-src`.
    #[must_use]
    pub fn script_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("script-src", sources)
    }

    /// Configures `style-src`.
    #[must_use]
    pub fn style_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("style-src", sources)
    }

    /// Configures `img-src`.
    #[must_use]
    pub fn img_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("img-src", sources)
    }

    /// Configures `font-src`.
    #[must_use]
    pub fn font_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("font-src", sources)
    }

    /// Configures `connect-src`.
    #[must_use]
    pub fn connect_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("connect-src", sources)
    }

    /// Configures `object-src`.
    #[must_use]
    pub fn object_src<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("object-src", sources)
    }

    /// Configures `frame-ancestors`.
    #[must_use]
    pub fn frame_ancestors<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("frame-ancestors", sources)
    }

    /// Configures `form-action`.
    #[must_use]
    pub fn form_action<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("form-action", sources)
    }

    /// Configures `base-uri`.
    #[must_use]
    pub fn base_uri<I, S>(self, sources: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.directive("base-uri", sources)
    }

    /// Adds the `upgrade-insecure-requests` directive.
    #[must_use]
    pub fn upgrade_insecure_requests(mut self) -> Self {
        self.directives
            .insert("upgrade-insecure-requests".to_string(), Vec::new());
        self
    }

    /// Adds the `block-all-mixed-content` directive.
    #[must_use]
    pub fn block_all_mixed_content(mut self) -> Self {
        self.directives
            .insert("block-all-mixed-content".to_string(), Vec::new());
        self
    }

    /// Finalizes and returns the [`CspConfig`].
    #[must_use]
    pub fn build(self) -> CspConfig {
        CspConfig {
            enabled: self.enabled,
            directives: self.directives,
            report_only: self.report_only,
            generate_nonce: self.generate_nonce,
            overwrite_existing: self.overwrite_existing,
        }
    }
}
