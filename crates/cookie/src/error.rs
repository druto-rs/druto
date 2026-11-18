//! Strongly typed error types for cookie validation, parsing, and cryptographic operations.

use std::fmt;

/// Errors produced during cookie modeling, header parsing, serialization, and cryptographic operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieError {
    /// The cookie name violates RFC 6265 token rules (e.g. contains separators, control characters, or is empty).
    InvalidName(String),
    /// The cookie value violates RFC 6265 rules (e.g. contains control characters, unquoted whitespace, or semicolons).
    InvalidValue(String),
    /// The `Cookie` or `Set-Cookie` header string is structurally malformed.
    MalformedHeader(String),
    /// HMAC signature verification failed or AEAD ciphertext was tampered with.
    AuthenticationFailed,
    /// Ciphertext was encrypted for a different cookie name (purpose mismatch substitution attack).
    PurposeMismatch,
    /// Attempted a signed or encrypted operation on an unkeyed cookie jar.
    MissingKey,
    /// Underlying cryptographic error from the `crypto` crate.
    Crypto(crypto::CryptoError),
    /// Base64 or UTF-8 string encoding/decoding failure.
    EncodingError(String),
}

impl fmt::Display for CookieError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(reason) => write!(formatter, "invalid cookie name: {reason}"),
            Self::InvalidValue(reason) => write!(formatter, "invalid cookie value: {reason}"),
            Self::MalformedHeader(reason) => {
                write!(formatter, "malformed cookie header string: {reason}")
            }
            Self::AuthenticationFailed => {
                write!(
                    formatter,
                    "cookie authentication failed: signature mismatch or ciphertext was tampered with"
                )
            }
            Self::PurposeMismatch => {
                write!(
                    formatter,
                    "cookie purpose mismatch: payload was encrypted for another cookie name"
                )
            }
            Self::MissingKey => {
                write!(
                    formatter,
                    "cannot sign or encrypt cookie: no encryption key or keyring was configured on the cookie jar"
                )
            }
            Self::Crypto(error) => write!(formatter, "cryptographic error: {error}"),
            Self::EncodingError(reason) => write!(formatter, "cookie encoding error: {reason}"),
        }
    }
}

impl std::error::Error for CookieError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Crypto(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crypto::CryptoError> for CookieError {
    fn from(error: crypto::CryptoError) -> Self {
        match error {
            crypto::CryptoError::AuthenticationFailed => Self::AuthenticationFailed,
            crypto::CryptoError::PurposeMismatch => Self::PurposeMismatch,
            other => Self::Crypto(other),
        }
    }
}

impl CookieError {
    /// Returns `true` if this error represents an invalid cookie name.
    #[must_use]
    pub fn is_invalid_name(&self) -> bool {
        matches!(self, Self::InvalidName(_))
    }

    /// Returns `true` if this error represents an invalid cookie value.
    #[must_use]
    pub fn is_invalid_value(&self) -> bool {
        matches!(self, Self::InvalidValue(_))
    }

    /// Returns `true` if this error represents a signature or authentication failure.
    #[must_use]
    pub fn is_authentication_failed(&self) -> bool {
        matches!(self, Self::AuthenticationFailed)
    }

    /// Returns `true` if this error represents a purpose binding substitution mismatch.
    #[must_use]
    pub fn is_purpose_mismatch(&self) -> bool {
        matches!(self, Self::PurposeMismatch)
    }

    /// Returns `true` if this error represents an unkeyed jar operation.
    #[must_use]
    pub fn is_missing_key(&self) -> bool {
        matches!(self, Self::MissingKey)
    }
}
