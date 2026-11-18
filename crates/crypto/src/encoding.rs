//! Internal Base64 decoding utilities supporting multiple engines and padding variants.

use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE_NO_PAD},
};

use crate::error::CryptoError;

/// Decodes Base64 input attempting URL-safe, standard padded, and unpadded variants.
pub(crate) fn decode_base64_flexible(encoded: &str) -> Result<Vec<u8>, CryptoError> {
    let trimmed = encoded.trim();
    URL_SAFE_NO_PAD
        .decode(trimmed)
        .or_else(|_| STANDARD.decode(trimmed))
        .or_else(|_| STANDARD_NO_PAD.decode(trimmed))
        .map_err(|err| CryptoError::EncodingError(err.to_string()))
}
