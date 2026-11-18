//! Cryptographic key validation, generation, and seamless key rotation container.

use std::fmt;

use base64::prelude::*;
use zeroize::Zeroize;

use crate::error::CryptoError;

/// Exact key length in bytes required for AES-256 and ChaCha20-Poly1305.
pub const KEY_LEN: usize = 32;

/// A validated 256-bit (32-byte) cryptographic key.
#[derive(Clone, PartialEq, Eq)]
pub struct Key([u8; KEY_LEN]);

impl Key {
    /// Creates a key from a raw 32-byte array.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    /// Attempts to create a key from an arbitrary byte slice, enforcing exact 32-byte length.
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, CryptoError> {
        if slice.len() != KEY_LEN {
            return Err(CryptoError::InvalidKeyLength {
                expected: KEY_LEN,
                actual: slice.len(),
            });
        }

        let mut buffer = [0u8; KEY_LEN];
        buffer.copy_from_slice(slice);
        Ok(Self(buffer))
    }

    /// Generates a fresh 256-bit key using the system CSPRNG.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = [0u8; KEY_LEN];
        getrandom::fill(&mut bytes)
            .map_err(|err| CryptoError::Internal(format!("CSPRNG key generation failed: {err}")))?;
        Ok(Self(bytes))
    }

    /// Decodes a 32-byte key from standard or URL-safe Base64.
    pub fn from_base64(encoded: &str) -> Result<Self, CryptoError> {
        let clean = encoded.trim();
        let stripped = clean.strip_prefix("base64:").unwrap_or(clean);

        let decoded = BASE64_STANDARD
            .decode(stripped)
            .or_else(|_| BASE64_URL_SAFE_NO_PAD.decode(stripped))
            .or_else(|_| BASE64_STANDARD_NO_PAD.decode(stripped))
            .map_err(|err| CryptoError::EncodingError(err.to_string()))?;

        Self::try_from_slice(&decoded)
    }

    /// Returns a reference to the internal 32-byte array.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// Returns a slice view of the key bytes.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Key([REDACTED])")
    }
}

impl Zeroize for Key {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Container holding a primary active key and zero or more historical fallback keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRing {
    primary: Key,
    previous: Vec<Key>,
}

impl KeyRing {
    /// Creates a key ring with only a primary key and no previous rotation keys.
    #[must_use]
    pub fn new(primary: Key) -> Self {
        Self {
            primary,
            previous: Vec::new(),
        }
    }

    /// Creates a key ring with a primary key and an ordered list of previous keys.
    #[must_use]
    pub fn with_previous(primary: Key, previous: Vec<Key>) -> Self {
        Self { primary, previous }
    }

    /// Returns a reference to the primary key used for all new encryption operations.
    #[must_use]
    pub const fn primary(&self) -> &Key {
        &self.primary
    }

    /// Returns the slice of fallback previous keys used during decryption.
    #[must_use]
    pub fn previous(&self) -> &[Key] {
        &self.previous
    }

    /// Iterates over all keys, beginning with the primary key followed by previous keys.
    pub fn all_keys(&self) -> impl Iterator<Item = &Key> {
        std::iter::once(&self.primary).chain(self.previous.iter())
    }
}
