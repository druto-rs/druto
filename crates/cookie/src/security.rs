//! Packing and unpacking helpers for plain, signed, and encrypted cookies.

use crypto::{Encrypter, HmacSigner};

use crate::error::CookieError;

/// Prefix identifying an HMAC signed cookie on the wire.
pub const SIGNED_PREFIX: &str = "s:";
/// Prefix identifying an AEAD encrypted cookie on the wire.
pub const ENCRYPTED_PREFIX: &str = "e:";

/// Security mode of a cookie value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CookieSecurity {
    /// Raw unencrypted, unsigned cookie.
    #[default]
    Plain,
    /// Tamper-proof HMAC signed cookie.
    Signed,
    /// Confidential and authentic AEAD encrypted cookie.
    Encrypted,
}

/// Detects the security mode of an encoded cookie string based on wire prefixes.
#[must_use]
pub fn detect_security(raw_value: &str) -> CookieSecurity {
    if raw_value.starts_with(ENCRYPTED_PREFIX) {
        CookieSecurity::Encrypted
    } else if raw_value.starts_with(SIGNED_PREFIX) {
        CookieSecurity::Signed
    } else {
        CookieSecurity::Plain
    }
}

/// Packs a signed cookie value bound to the cookie name.
#[must_use]
pub fn pack_signed(name: &str, value: &str, signer: &HmacSigner) -> String {
    let payload = format!("cookie:{name}:{value}");
    let signature = signer.sign_base64(payload.as_bytes());
    format!("{SIGNED_PREFIX}{value}.{signature}")
}

/// Unpacks and verifies a signed cookie value bound to the cookie name.
pub fn unpack_signed(name: &str, raw: &str, signer: &HmacSigner) -> Result<String, CookieError> {
    let stripped = raw
        .strip_prefix(SIGNED_PREFIX)
        .ok_or(CookieError::AuthenticationFailed)?;

    let (value, signature) = stripped
        .rsplit_once('.')
        .ok_or(CookieError::AuthenticationFailed)?;

    let payload = format!("cookie:{name}:{value}");
    let is_valid = signer.verify_base64(payload.as_bytes(), signature);

    if is_valid {
        Ok(value.to_string())
    } else {
        Err(CookieError::AuthenticationFailed)
    }
}

/// Packs an encrypted cookie value bound to the cookie name via AEAD AAD.
pub fn pack_encrypted(
    name: &str,
    value: &str,
    encrypter: &Encrypter,
) -> Result<String, CookieError> {
    let purpose = format!("cookie:{name}");
    let envelope_b64 = encrypter.encrypt_string_with_purpose(value, &purpose)?;
    Ok(format!("{ENCRYPTED_PREFIX}{envelope_b64}"))
}

/// Unpacks and decrypts an encrypted cookie value verifying purpose binding.
pub fn unpack_encrypted(
    name: &str,
    raw: &str,
    encrypter: &Encrypter,
) -> Result<String, CookieError> {
    let envelope_b64 = raw
        .strip_prefix(ENCRYPTED_PREFIX)
        .ok_or(CookieError::AuthenticationFailed)?;

    let purpose = format!("cookie:{name}");
    encrypter
        .decrypt_string_with_purpose(envelope_b64, &purpose)
        .map_err(CookieError::from)
}

/// Unpacks and verifies a signed cookie value against any key in the keyring.
pub fn unpack_signed_with_keyring(
    name: &str,
    raw: &str,
    keyring: &crypto::KeyRing,
) -> Result<String, CookieError> {
    let stripped = raw
        .strip_prefix(SIGNED_PREFIX)
        .ok_or(CookieError::AuthenticationFailed)?;

    let (value, signature) = stripped
        .rsplit_once('.')
        .ok_or(CookieError::AuthenticationFailed)?;

    let payload = format!("cookie:{name}:{value}");
    let payload_bytes = payload.as_bytes();

    for key in keyring.all_keys() {
        let signer = HmacSigner::new(key.clone());
        if signer.verify_base64(payload_bytes, signature) {
            return Ok(value.to_string());
        }
    }

    Err(CookieError::AuthenticationFailed)
}

/// Unpacks and decrypts an encrypted cookie value using the keyring.
pub fn unpack_encrypted_with_keyring(
    name: &str,
    raw: &str,
    keyring: &crypto::KeyRing,
) -> Result<String, CookieError> {
    let encrypter = Encrypter::new(keyring.clone());
    unpack_encrypted(name, raw, &encrypter)
}
