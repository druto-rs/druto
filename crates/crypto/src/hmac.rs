//! Constant-time HMAC-SHA256 signature generation and verification.

use base64::prelude::*;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::key::Key;

type HmacSha256 = Hmac<Sha256>;

/// Length of an HMAC-SHA256 output digest in bytes.
pub const HMAC_SHA256_LEN: usize = 32;

/// HMAC-SHA256 signer and verifier providing timing side-channel resistant verification.
#[derive(Clone, Debug)]
pub struct HmacSigner {
    key: Key,
}

impl HmacSigner {
    /// Creates a new HMAC-SHA256 signer bound to the provided key.
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self { key }
    }

    /// Returns a reference to the signing key.
    #[must_use]
    pub const fn key(&self) -> &Key {
        &self.key
    }

    /// Computes the 32-byte HMAC-SHA256 signature for the given input data.
    #[must_use]
    pub fn sign(&self, data: &[u8]) -> [u8; HMAC_SHA256_LEN] {
        let mut mac =
            HmacSha256::new_from_slice(self.key.as_slice()).expect("key is verified 32 bytes");
        mac.update(data);
        let output = mac.finalize().into_bytes();

        let mut signature = [0u8; HMAC_SHA256_LEN];
        signature.copy_from_slice(&output);
        signature
    }

    /// Computes the HMAC-SHA256 signature and encodes it as an unpadded URL-safe Base64 string.
    #[must_use]
    pub fn sign_base64(&self, data: &[u8]) -> String {
        let signature = self.sign(data);
        BASE64_URL_SAFE_NO_PAD.encode(signature)
    }

    /// Verifies the given data against an expected signature using constant-time comparison.
    #[must_use]
    pub fn verify(&self, data: &[u8], expected_signature: &[u8]) -> bool {
        let computed = self.sign(data);
        if expected_signature.len() != computed.len() {
            return false;
        }

        expected_signature.ct_eq(&computed).into()
    }

    /// Verifies data against a Base64URL-encoded signature using constant-time comparison.
    #[must_use]
    pub fn verify_base64(&self, data: &[u8], signature_base64: &str) -> bool {
        let trimmed = signature_base64.trim();
        let decoded = match BASE64_URL_SAFE_NO_PAD
            .decode(trimmed)
            .or_else(|_| BASE64_STANDARD_NO_PAD.decode(trimmed))
            .or_else(|_| BASE64_STANDARD.decode(trimmed))
        {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        self.verify(data, &decoded)
    }
}
