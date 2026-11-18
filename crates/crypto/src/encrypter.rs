//! High-level AEAD encrypter with key rotation, purpose binding, and Base64URL envelopes.

use aes_gcm::{
    Aes256Gcm, Nonce as AesNonce,
    aead::{Aead, KeyInit, Payload},
};
use chacha20poly1305::{ChaCha20Poly1305, Nonce as ChaChaNonce};

use crate::{
    algorithm::Algorithm,
    envelope::{Envelope, NONCE_LEN, TAG_LEN},
    error::CryptoError,
    key::{Key, KeyRing},
};

/// High-level encrypter managing AEAD ciphers, key rotation, and context purpose binding.
#[derive(Clone, Debug)]
pub struct Encrypter {
    keyring: KeyRing,
    algorithm: Algorithm,
}

impl Encrypter {
    /// Creates a new encrypter using the given key ring and default cipher (AES-256-GCM).
    #[must_use]
    pub fn new(keyring: KeyRing) -> Self {
        Self {
            keyring,
            algorithm: Algorithm::default(),
        }
    }

    /// Creates a new encrypter with an explicitly configured cipher algorithm.
    #[must_use]
    pub fn with_algorithm(keyring: KeyRing, algorithm: Algorithm) -> Self {
        Self { keyring, algorithm }
    }

    /// Creates an encrypter from a single primary key without rotation keys.
    #[must_use]
    pub fn from_key(key: Key) -> Self {
        Self::new(KeyRing::new(key))
    }

    /// Returns a reference to the active key ring.
    #[must_use]
    pub const fn keyring(&self) -> &KeyRing {
        &self.keyring
    }

    /// Returns the cipher algorithm used for new encryptions.
    #[must_use]
    pub const fn algorithm(&self) -> Algorithm {
        self.algorithm
    }

    /// Encrypts raw byte plaintext without purpose binding.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, CryptoError> {
        self.encrypt_with_purpose(plaintext, "")
    }

    /// Encrypts a UTF-8 string without purpose binding.
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String, CryptoError> {
        self.encrypt_with_purpose(plaintext.as_bytes(), "")
    }

    /// Encrypts a UTF-8 string bound cryptographically to a specific context purpose.
    pub fn encrypt_string_with_purpose(
        &self,
        plaintext: &str,
        purpose: &str,
    ) -> Result<String, CryptoError> {
        self.encrypt_with_purpose(plaintext.as_bytes(), purpose)
    }

    /// Encrypts raw bytes bound cryptographically to a specific context purpose.
    pub fn encrypt_with_purpose(
        &self,
        plaintext: &[u8],
        purpose: &str,
    ) -> Result<String, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce_bytes).map_err(|err| {
            CryptoError::Internal(format!("nonce CSPRNG generation failed: {err}"))
        })?;

        let primary_key = self.keyring.primary();
        let aad = purpose.as_bytes();

        let encrypted = match self.algorithm {
            Algorithm::Aes256Gcm => {
                let cipher = Aes256Gcm::new_from_slice(primary_key.as_slice())
                    .expect("key is verified 32 bytes");
                let nonce = AesNonce::from(nonce_bytes);
                cipher
                    .encrypt(
                        &nonce,
                        Payload {
                            msg: plaintext,
                            aad,
                        },
                    )
                    .map_err(|_| CryptoError::AuthenticationFailed)?
            }
            Algorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new_from_slice(primary_key.as_slice())
                    .expect("key is verified 32 bytes");
                let nonce = ChaChaNonce::from(nonce_bytes);
                cipher
                    .encrypt(
                        &nonce,
                        Payload {
                            msg: plaintext,
                            aad,
                        },
                    )
                    .map_err(|_| CryptoError::AuthenticationFailed)?
            }
        };

        if encrypted.len() < TAG_LEN {
            return Err(CryptoError::Internal(
                "cipher output shorter than tag length".to_string(),
            ));
        }

        let tag_offset = encrypted.len() - TAG_LEN;
        let mut tag_bytes = [0u8; TAG_LEN];
        tag_bytes.copy_from_slice(&encrypted[tag_offset..]);
        let ciphertext = encrypted[..tag_offset].to_vec();

        let envelope = Envelope::new(self.algorithm, nonce_bytes, tag_bytes, ciphertext);
        Ok(envelope.to_base64())
    }

    /// Decrypts a URL-safe Base64 envelope without purpose binding.
    pub fn decrypt(&self, encoded: &str) -> Result<Vec<u8>, CryptoError> {
        self.decrypt_with_purpose(encoded, "")
    }

    /// Decrypts a URL-safe Base64 envelope to a UTF-8 string without purpose binding.
    pub fn decrypt_string(&self, encoded: &str) -> Result<String, CryptoError> {
        let bytes = self.decrypt_with_purpose(encoded, "")?;
        String::from_utf8(bytes)
            .map_err(|err| CryptoError::EncodingError(format!("UTF-8 decode failure: {err}")))
    }

    /// Decrypts a URL-safe Base64 envelope to a UTF-8 string with purpose binding verification.
    pub fn decrypt_string_with_purpose(
        &self,
        encoded: &str,
        purpose: &str,
    ) -> Result<String, CryptoError> {
        let bytes = self.decrypt_with_purpose(encoded, purpose)?;
        String::from_utf8(bytes)
            .map_err(|err| CryptoError::EncodingError(format!("UTF-8 decode failure: {err}")))
    }

    /// Decrypts a URL-safe Base64 envelope verifying purpose binding against the active key ring.
    pub fn decrypt_with_purpose(
        &self,
        encoded: &str,
        purpose: &str,
    ) -> Result<Vec<u8>, CryptoError> {
        let envelope = Envelope::from_base64(encoded)?;
        let aad = purpose.as_bytes();

        let mut combined = Vec::with_capacity(envelope.ciphertext.len() + TAG_LEN);
        combined.extend_from_slice(&envelope.ciphertext);
        combined.extend_from_slice(&envelope.tag);

        for key in self.keyring.all_keys() {
            let decrypt_result = match envelope.algorithm {
                Algorithm::Aes256Gcm => {
                    let cipher = Aes256Gcm::new_from_slice(key.as_slice())
                        .expect("key is verified 32 bytes");
                    let nonce = AesNonce::from(envelope.nonce);
                    cipher.decrypt(
                        &nonce,
                        Payload {
                            msg: &combined,
                            aad,
                        },
                    )
                }
                Algorithm::ChaCha20Poly1305 => {
                    let cipher = ChaCha20Poly1305::new_from_slice(key.as_slice())
                        .expect("key is verified 32 bytes");
                    let nonce = ChaChaNonce::from(envelope.nonce);
                    cipher.decrypt(
                        &nonce,
                        Payload {
                            msg: &combined,
                            aad,
                        },
                    )
                }
            };

            if let Ok(plaintext) = decrypt_result {
                return Ok(plaintext);
            }
        }

        Err(CryptoError::AuthenticationFailed)
    }
}
