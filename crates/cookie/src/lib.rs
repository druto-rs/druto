//! RFC 6265 compliant HTTP cookie modeling, parsing, serialization, and high-level cookie jars.
//!
//! Provides three operational security modes—plain, tamper-proof signed, and confidential
//! AEAD authenticated encrypted—with read-your-own-writes overlay mutation tracking.

pub mod config;
pub mod cookie;
pub mod error;
pub mod jar;
pub mod same_site;
pub mod security;

pub use config::CookieConfig;
pub use cookie::{Cookie, CookieBuilder};
pub use error::CookieError;
pub use jar::CookieJar;
pub use same_site::SameSite;
pub use security::CookieSecurity;
