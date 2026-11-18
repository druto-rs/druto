//! Cryptographic primitives, AEAD encryption, HMAC signing, and key rotation for Druto.
//!
//! Provides authenticated encryption ([`Encrypter`]) supporting AES-256-GCM and ChaCha20-Poly1305,
//! zero-downtime key rotation ([`KeyRing`]), timing-safe HMAC signing ([`HmacSigner`]), and canonical
//! signed URL generation and validation ([`UrlSigner`]).

pub mod algorithm;
pub(crate) mod encoding;
pub mod encrypter;
pub mod envelope;
pub mod error;
pub mod hmac;
pub mod key;
pub mod url_signer;

pub use crate::{
    algorithm::Algorithm,
    encrypter::Encrypter,
    envelope::{ALG_LEN, Envelope, HEADER_LEN, NONCE_LEN, TAG_LEN},
    error::CryptoError,
    hmac::{HMAC_SHA256_LEN, HmacSigner},
    key::{KEY_LEN, Key, KeyRing},
    url_signer::UrlSigner,
};
