//! Compact binary and URL-safe Base64 serialization envelope for AEAD payloads.

use base64::prelude::*;

use crate::{algorithm::Algorithm, error::CryptoError};

/// Number of bytes allocated for the algorithm identifier.
pub const ALG_LEN: usize = 1;
/// Number of bytes for the standard 96-bit AEAD nonce.
pub const NONCE_LEN: usize = 12;
/// Number of bytes for the 128-bit authentication tag.
pub const TAG_LEN: usize = 16;
/// Total size of the fixed binary envelope header (29 bytes).
pub const HEADER_LEN: usize = ALG_LEN + NONCE_LEN + TAG_LEN;

/// Encapsulation of ciphertext alongside cipher algorithm, nonce, and authentication tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// Cipher algorithm used for encryption.
    pub algorithm: Algorithm,
    /// 96-bit cryptographically secure random nonce.
    pub nonce: [u8; NONCE_LEN],
    /// 128-bit authentication tag.
    pub tag: [u8; TAG_LEN],
    /// Encrypted raw ciphertext bytes.
    pub ciphertext: Vec<u8>,
}

impl Envelope {
    /// Creates a new envelope instance.
    #[must_use]
    pub const fn new(
        algorithm: Algorithm,
        nonce: [u8; NONCE_LEN],
        tag: [u8; TAG_LEN],
        ciphertext: Vec<u8>,
    ) -> Self {
        Self {
            algorithm,
            nonce,
            tag,
            ciphertext,
        }
    }

    /// Serializes the envelope into a compact binary byte vector.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(HEADER_LEN + self.ciphertext.len());
        buffer.push(self.algorithm.code());
        buffer.extend_from_slice(&self.nonce);
        buffer.extend_from_slice(&self.tag);
        buffer.extend_from_slice(&self.ciphertext);
        buffer
    }

    /// Deserializes an envelope from a raw binary byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() < HEADER_LEN {
            return Err(CryptoError::MalformedEnvelope(format!(
                "payload is shorter than minimal header length of {HEADER_LEN} bytes (got {})",
                bytes.len()
            )));
        }

        let algorithm = Algorithm::from_code(bytes[0])?;

        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&bytes[ALG_LEN..ALG_LEN + NONCE_LEN]);

        let tag_start = ALG_LEN + NONCE_LEN;
        let mut tag = [0u8; TAG_LEN];
        tag.copy_from_slice(&bytes[tag_start..tag_start + TAG_LEN]);

        let ciphertext = bytes[HEADER_LEN..].to_vec();

        Ok(Self {
            algorithm,
            nonce,
            tag,
            ciphertext,
        })
    }

    /// Serializes the envelope into an unpadded URL-safe Base64 string.
    #[must_use]
    pub fn to_base64(&self) -> String {
        let bytes = self.to_bytes();
        BASE64_URL_SAFE_NO_PAD.encode(bytes)
    }

    /// Deserializes an envelope from an unpadded URL-safe Base64 string.
    pub fn from_base64(encoded: &str) -> Result<Self, CryptoError> {
        let bytes = crate::encoding::decode_base64_flexible(encoded)?;
        Self::from_bytes(&bytes)
    }
}
