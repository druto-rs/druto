//! Cross-Origin Resource Sharing (CORS) preflight and response header middleware for Druto.
//!
//! Provides automated preflight negotiation, origin validation, and response header injection
//! compliant with the W3C CORS specification and RFC 6454.
//!
//! # Examples
//!
//! ```rust
//! use cors::{CorsConfig, CorsMiddleware, OriginPolicy};
//!
//! let config = CorsConfig::builder()
//!     .allow_origin(OriginPolicy::pattern(["https://*.example.com"]))
//!     .allow_credentials(true)
//!     .max_age(300)
//!     .build();
//!
//! let middleware = CorsMiddleware::new(config);
//! ```

pub mod config;
pub mod constants;
pub mod middleware;

pub use config::{
    CorsConfig, CorsConfigBuilder, HeaderPolicy, MethodPolicy, OriginPolicy, OriginPredicate,
    matches_path,
};
pub use constants::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE,
    ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ORIGIN, SIMPLE_EXPOSE_HEADERS,
    VARY,
};
pub use middleware::CorsMiddleware;

/// Common types and builders for CORS configuration.
pub mod prelude {
    pub use crate::{
        CorsConfig, CorsConfigBuilder, CorsMiddleware, HeaderPolicy, MethodPolicy, OriginPolicy,
    };
}
