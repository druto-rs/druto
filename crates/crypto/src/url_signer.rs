//! Canonical URL signing and tamper-proof temporary signed URLs.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{error::CryptoError, hmac::HmacSigner, key::Key};

/// URL signer and validator providing canonical query string signing and expiry validation.
#[derive(Clone, Debug)]
pub struct UrlSigner {
    signer: HmacSigner,
}

impl UrlSigner {
    /// Creates a new URL signer bound to the provided key.
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self {
            signer: HmacSigner::new(key),
        }
    }

    /// Signs a URL by appending a `signature` query parameter.
    #[must_use]
    pub fn sign(&self, url: &str) -> String {
        let canonical = Self::canonicalize(url, None);
        let signature = self.signer.sign_base64(canonical.as_bytes());

        if url.contains('?') {
            format!("{url}&signature={signature}")
        } else {
            format!("{url}?signature={signature}")
        }
    }

    /// Signs a URL with an expiration timestamp by appending `expires` and `signature` query parameters.
    #[must_use]
    pub fn sign_temporary(&self, url: &str, expires_at_unix: u64) -> String {
        let canonical = Self::canonicalize(url, Some(expires_at_unix));
        let signature = self.signer.sign_base64(canonical.as_bytes());

        if url.contains('?') {
            format!("{url}&expires={expires_at_unix}&signature={signature}")
        } else {
            format!("{url}?expires={expires_at_unix}&signature={signature}")
        }
    }

    /// Verifies a signed URL against the current system clock.
    pub fn verify(&self, signed_url: &str) -> Result<(), CryptoError> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| CryptoError::Internal(format!("system clock error: {err}")))?
            .as_secs();

        self.verify_at(signed_url, current_time)
    }

    /// Verifies a signed URL against an explicit Unix timestamp in seconds.
    pub fn verify_at(&self, signed_url: &str, current_time_unix: u64) -> Result<(), CryptoError> {
        let (base_path, query_str) = match signed_url.split_once('?') {
            Some((base, query)) => (base, query),
            None => {
                return Err(CryptoError::MissingParameter("signature".to_string()));
            }
        };

        let mut signature_opt: Option<String> = None;
        let mut expires_opt: Option<u64> = None;
        let mut other_params: Vec<(String, String)> = Vec::new();

        for pair in query_str.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (key, value) = match pair.split_once('=') {
                Some((k, v)) => (k, v),
                None => (pair, ""),
            };

            if key == "signature" {
                signature_opt = Some(value.to_string());
            } else if key == "expires" {
                let expires_val = value.parse::<u64>().map_err(|_| {
                    CryptoError::MalformedEnvelope("invalid expires timestamp format".to_string())
                })?;
                expires_opt = Some(expires_val);
                other_params.push((key.to_string(), value.to_string()));
            } else {
                other_params.push((key.to_string(), value.to_string()));
            }
        }

        let signature =
            signature_opt.ok_or_else(|| CryptoError::MissingParameter("signature".to_string()))?;

        // Canonical query sorting
        other_params.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

        let canonical = if other_params.is_empty() {
            base_path.to_string()
        } else {
            let sorted_query = other_params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            format!("{base_path}?{sorted_query}")
        };

        // Authenticity must be cryptographically verified prior to semantic expiration checks
        let is_valid = self.signer.verify_base64(canonical.as_bytes(), &signature);
        if !is_valid {
            return Err(CryptoError::InvalidSignature);
        }

        if let Some(expires) = expires_opt.filter(|&expires| current_time_unix > expires) {
            return Err(CryptoError::ExpiredSignature {
                expires,
                current: current_time_unix,
            });
        }

        Ok(())
    }

    fn canonicalize(url: &str, expires_opt: Option<u64>) -> String {
        let (base_path, query_str) = match url.split_once('?') {
            Some((base, query)) => (base, Some(query)),
            None => (url, None),
        };

        let mut params: Vec<(String, String)> = Vec::new();

        if let Some(query) = query_str {
            for pair in query.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let (key, value) = match pair.split_once('=') {
                    Some((k, v)) => (k, v),
                    None => (pair, ""),
                };

                if key != "signature" && key != "expires" {
                    params.push((key.to_string(), value.to_string()));
                }
            }
        }

        if let Some(expires) = expires_opt {
            params.push(("expires".to_string(), expires.to_string()));
        }

        params.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

        if params.is_empty() {
            base_path.to_string()
        } else {
            let query = params
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            format!("{base_path}?{query}")
        }
    }
}
