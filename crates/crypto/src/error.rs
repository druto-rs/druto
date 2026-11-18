//! Strongly typed errors for cryptographic operations, key rotation, and URL signing.

use std::fmt;

/// Errors produced during encryption, decryption, key validation, and signature verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Provided key does not satisfy the required 256-bit (32-byte) length.
    InvalidKeyLength {
        /// Expected byte length.
        expected: usize,
        /// Actual byte length provided.
        actual: usize,
    },
    /// Ciphertext authentication tag check failed or data was tampered with.
    AuthenticationFailed,
    /// Ciphertext envelope structure or serialization format is invalid.
    MalformedEnvelope(String),
    /// URL-safe Base64 encoding or decoding failed.
    EncodingError(String),
    /// Ciphertext purpose binding does not match the expected purpose string.
    ///
    /// Note: Standard AEAD ciphers (AES-256-GCM and ChaCha20-Poly1305) authenticate
    /// the purpose string as Additional Authenticated Data (AAD). A mismatch in purpose
    /// causes authentication tag verification to fail, returning [`CryptoError::AuthenticationFailed`].
    /// This variant is reserved for explicit application-level purpose validations.
    PurposeMismatch,
    /// Temporary signed URL timestamp has expired.
    ExpiredSignature {
        /// The expiration unix timestamp recorded in the URL.
        expires: u64,
        /// The current unix timestamp at validation time.
        current: u64,
    },
    /// Signature verification failed due to alteration or mismatched key.
    InvalidSignature,
    /// Required query parameter is missing from the signed URL.
    MissingParameter(String),
    /// Underlying entropy generator or system RNG failed.
    Internal(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKeyLength { expected, actual } => {
                write!(
                    formatter,
                    "invalid key length: expected {expected} bytes, got {actual} bytes"
                )
            }
            Self::AuthenticationFailed => {
                write!(
                    formatter,
                    "cryptographic authentication failed: ciphertext was tampered with or key is invalid"
                )
            }
            Self::MalformedEnvelope(reason) => {
                write!(formatter, "malformed encryption envelope: {reason}")
            }
            Self::EncodingError(reason) => write!(formatter, "base64 encoding error: {reason}"),
            Self::PurposeMismatch => {
                write!(
                    formatter,
                    "purpose mismatch: ciphertext was encrypted for a different purpose"
                )
            }
            Self::ExpiredSignature { expires, current } => {
                write!(
                    formatter,
                    "signed URL has expired (expires at {expires}, current time is {current})"
                )
            }
            Self::InvalidSignature => write!(formatter, "invalid cryptographic signature"),
            Self::MissingParameter(name) => {
                write!(formatter, "missing parameter in signed URL: '{name}'")
            }
            Self::Internal(reason) => {
                write!(formatter, "internal cryptographic failure: {reason}")
            }
        }
    }
}

impl std::error::Error for CryptoError {}

impl CryptoError {
    /// Returns `true` if this error represents an authentication tag failure.
    #[must_use]
    pub fn is_authentication_failed(&self) -> bool {
        matches!(self, Self::AuthenticationFailed)
    }

    /// Returns `true` if this error represents an expired temporary signature.
    #[must_use]
    pub fn is_expired_signature(&self) -> bool {
        matches!(self, Self::ExpiredSignature { .. })
    }

    /// Returns `true` if this error represents an invalid signature.
    #[must_use]
    pub fn is_invalid_signature(&self) -> bool {
        matches!(self, Self::InvalidSignature)
    }

    /// Returns `true` if this error represents a purpose mismatch.
    #[must_use]
    pub fn is_purpose_mismatch(&self) -> bool {
        matches!(self, Self::PurposeMismatch)
    }

    /// Returns `true` if this error represents a malformed serialization envelope.
    #[must_use]
    pub fn is_malformed_envelope(&self) -> bool {
        matches!(self, Self::MalformedEnvelope(_))
    }
}
