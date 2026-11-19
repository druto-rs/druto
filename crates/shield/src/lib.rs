//! HTTP web application security hardening, request guard middlewares, and security header enforcement for Druto.
//!
//! Compliant with OWASP guidelines, RFC 6797 (HSTS), RFC 7034 (Frame Options), and W3C specifications.
//!
//! # Examples
//!
//! ## Unified Security Middleware
//!
//! ```rust
//! use shield::{CspConfig, CsrfConfig, SecurityPreset, ShieldConfig, ShieldMiddleware};
//!
//! let shield = ShieldMiddleware::new(
//!     ShieldConfig::builder()
//!         .preset(SecurityPreset::SecureDefaults)
//!         .csp(
//!             CspConfig::builder()
//!                 .default_src(vec!["'self'"])
//!                 .script_src(vec!["'self'", "@nonce"])
//!                 .generate_nonce(true)
//!                 .build(),
//!         )
//!         .csrf(CsrfConfig::default())
//!         .build(),
//! );
//! ```
//!
//! ## Standalone Security Headers
//!
//! ```rust
//! use shield::{FrameAction, ReferrerPolicy, SecurityHeadersConfig, SecurityHeadersMiddleware};
//!
//! let middleware = SecurityHeadersMiddleware::from_builder(
//!     SecurityHeadersConfig::builder()
//!         .frame_guard(Some(FrameAction::Deny))
//!         .referrer_policy(Some(ReferrerPolicy::NoReferrer))
//!         .no_sniff(true),
//! );
//! ```

pub mod constants;
pub mod csp;
pub mod csrf;
pub mod error;
pub mod headers;
pub mod middleware;

pub use constants::*;
pub use csp::{CspConfig, CspConfigBuilder, CspNonce};
pub use csrf::{
    CsrfConfig, CsrfConfigBuilder, CsrfFailureMode, CsrfToken, extract_csrf_token,
    generate_csrf_token, verify_csrf_token,
};
pub use error::ShieldError;
pub use headers::{
    CrossDomainPolicy, FrameAction, HstsConfig, PermissionsPolicyBuilder, ReferrerPolicy,
    SecurityHeadersConfig, SecurityHeadersConfigBuilder, SecurityPreset,
};
pub use middleware::{
    CspMiddleware, CsrfMiddleware, SecurityHeadersMiddleware, ShieldConfig, ShieldConfigBuilder,
    ShieldMiddleware,
};

/// Convenient prelude re-exporting key types and guards.
pub mod prelude {
    pub use crate::{
        constants::*,
        csp::{CspConfig, CspNonce},
        csrf::{CsrfConfig, CsrfFailureMode, CsrfToken},
        error::ShieldError,
        headers::{
            CrossDomainPolicy, FrameAction, HstsConfig, PermissionsPolicyBuilder, ReferrerPolicy,
            SecurityHeadersConfig, SecurityPreset,
        },
        middleware::{
            CspMiddleware, CsrfMiddleware, SecurityHeadersMiddleware, ShieldConfig,
            ShieldMiddleware,
        },
    };
}
