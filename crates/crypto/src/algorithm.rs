//! Supported authenticated encryption with associated data (AEAD) algorithms.

use std::fmt;

use crate::error::CryptoError;

/// AEAD ciphers supported by the encryption subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Algorithm {
    /// AES-256 in Galois/Counter Mode (hardware-accelerated via AES-NI / ARMv8 Crypto).
    #[default]
    Aes256Gcm = 1,
    /// ChaCha20 stream cipher with Poly1305 MAC (high-performance pure software cipher).
    ChaCha20Poly1305 = 2,
}

impl Algorithm {
    /// Returns the binary identifier byte used in serialized envelopes.
    #[must_use]
    pub const fn code(&self) -> u8 {
        *self as u8
    }

    /// Resolves an algorithm from its binary identifier byte.
    pub fn from_code(code: u8) -> Result<Self, CryptoError> {
        match code {
            1 => Ok(Self::Aes256Gcm),
            2 => Ok(Self::ChaCha20Poly1305),
            other => Err(CryptoError::MalformedEnvelope(format!(
                "unsupported algorithm code: {other}"
            ))),
        }
    }
}

impl fmt::Display for Algorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aes256Gcm => write!(formatter, "aes-256-gcm"),
            Self::ChaCha20Poly1305 => write!(formatter, "chacha20-poly1305"),
        }
    }
}
