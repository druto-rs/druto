//! Error taxonomy for web application security hardening and request guards.

use std::fmt;

use router::RouterError;

/// Errors produced during security verification or CSRF token validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShieldError {
    /// A required CSRF token was not supplied in headers, body, or query parameters.
    MissingCsrfToken,
    /// The supplied CSRF token was malformed, expired, or failed cryptographic verification.
    InvalidCsrfToken,
    /// The user session or CSRF secret could not be resolved from the request context.
    MissingCsrfSecret,
    /// An invalid configuration was supplied.
    InvalidConfiguration(String),
}

impl fmt::Display for ShieldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCsrfToken => {
                formatter.write_str("403 Forbidden: Missing required CSRF token")
            }
            Self::InvalidCsrfToken => {
                formatter.write_str("403 Forbidden: Invalid or expired CSRF token")
            }
            Self::MissingCsrfSecret => {
                formatter.write_str("500 Internal Server Error: Missing CSRF secret in session")
            }
            Self::InvalidConfiguration(reason) => {
                write!(formatter, "Invalid shield configuration: {reason}")
            }
        }
    }
}

impl std::error::Error for ShieldError {}

impl From<ShieldError> for RouterError {
    fn from(error: ShieldError) -> Self {
        match error {
            ShieldError::MissingCsrfToken | ShieldError::InvalidCsrfToken => Self::InvalidSignature,
            ShieldError::MissingCsrfSecret | ShieldError::InvalidConfiguration(_) => {
                Self::Internal(error.to_string())
            }
        }
    }
}
