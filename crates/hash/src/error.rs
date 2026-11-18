//! Strongly typed errors for password hashing, verification, and driver resolution.

use std::fmt;

/// Errors produced during password hashing, verification, and driver resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashError {
    /// The provided hash string is malformed, truncated, or structurally invalid.
    InvalidHash(String),
    /// Cost parameters or configuration options are outside acceptable bounds.
    InvalidParameters(String),
    /// A requested hashing driver was not found in the manager registry.
    DriverNotFound(String),
    /// An internal cryptographic or backend hashing failure occurred.
    Internal(String),
}

impl fmt::Display for HashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHash(reason) => write!(formatter, "invalid hash string: {reason}"),
            Self::InvalidParameters(reason) => {
                write!(formatter, "invalid hash parameters: {reason}")
            }
            Self::DriverNotFound(name) => {
                write!(formatter, "hash driver not found: '{name}'")
            }
            Self::Internal(reason) => write!(formatter, "internal hashing failure: {reason}"),
        }
    }
}

impl std::error::Error for HashError {}

impl HashError {
    /// Returns `true` if this error represents an invalid or malformed hash string.
    #[must_use]
    pub fn is_invalid_hash(&self) -> bool {
        matches!(self, Self::InvalidHash(_))
    }

    /// Returns `true` if this error represents an invalid configuration or cost parameter.
    #[must_use]
    pub fn is_invalid_parameters(&self) -> bool {
        matches!(self, Self::InvalidParameters(_))
    }

    /// Returns `true` if this error represents an unregistered driver name.
    #[must_use]
    pub fn is_driver_not_found(&self) -> bool {
        matches!(self, Self::DriverNotFound(_))
    }

    /// Returns `true` if this error represents an internal backend failure.
    #[must_use]
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }
}
