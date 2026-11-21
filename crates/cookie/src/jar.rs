//! Request cookie inspection and response cookie staging area with an overlay mutation model.

use std::collections::BTreeMap;

use gopon::{Encrypter, HmacSigner, KeyRing};
use time::{Duration, OffsetDateTime};

use crate::{
    config::CookieConfig,
    cookie::Cookie,
    error::CookieError,
    security::{
        CookieSecurity, detect_security, pack_encrypted, pack_signed,
        unpack_encrypted_with_keyring, unpack_signed_with_keyring,
    },
};

/// High-level cookie container providing read-your-own-writes semantics and delta tracking.
#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    incoming: BTreeMap<String, String>,
    outgoing: BTreeMap<String, Cookie>,
    config: CookieConfig,
    keyring: Option<KeyRing>,
}

impl CookieJar {
    /// Creates an empty cookie jar with default configuration and without a cryptographic key ring.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a cookie jar populated from an incoming HTTP `Cookie` header string.
    #[must_use]
    pub fn from_headers(header: Option<&str>) -> Self {
        let mut jar = Self::default();
        if let Some(header_str) = header {
            for (name, value) in Cookie::parse_cookie_header(header_str) {
                jar.incoming.insert(name, value);
            }
        }
        jar
    }

    /// Creates a cookie jar with an attached cryptographic key ring.
    #[must_use]
    pub fn with_keyring(keyring: KeyRing) -> Self {
        Self {
            keyring: Some(keyring),
            ..Self::default()
        }
    }

    /// Creates a cookie jar populated from a `Cookie` header string with a cryptographic key ring.
    #[must_use]
    pub fn from_headers_with_keyring(header: Option<&str>, keyring: KeyRing) -> Self {
        let mut jar = Self::from_headers(header);
        jar.keyring = Some(keyring);
        jar
    }

    /// Attaches or replaces the cryptographic key ring.
    pub fn set_keyring(&mut self, keyring: impl Into<Option<KeyRing>>) {
        self.keyring = keyring.into();
    }

    /// Returns a reference to the active key ring, if configured.
    #[must_use]
    pub const fn keyring(&self) -> Option<&KeyRing> {
        self.keyring.as_ref()
    }

    /// Sets the jar configuration policy.
    pub fn set_config(&mut self, config: CookieConfig) {
        self.config = config;
    }

    /// Builder method to set the jar configuration policy.
    #[must_use]
    pub fn with_config(mut self, config: CookieConfig) -> Self {
        self.config = config;
        self
    }

    /// Returns a reference to the active configuration policy.
    #[must_use]
    pub const fn config(&self) -> &CookieConfig {
        &self.config
    }

    /// Returns a mutable reference to the configuration policy.
    pub fn config_mut(&mut self) -> &mut CookieConfig {
        &mut self.config
    }

    /// Returns the decrypted or verified cookie value, auto-detecting the security mode.
    ///
    /// Reads staged outgoing mutations first (read-your-own-writes), falling back to incoming
    /// request cookies. If the cookie was tombstoned/removed in `outgoing`, returns `Ok(None)`.
    pub fn get(&self, name: &str) -> Result<Option<String>, CookieError> {
        if let Some(cookie) = self.outgoing.get(name) {
            if cookie.is_tombstone() {
                return Ok(None);
            }
            return self.decode_value(name, cookie.value()).map(Some);
        }

        if let Some(raw) = self.incoming.get(name) {
            return self.decode_value(name, raw).map(Some);
        }

        Ok(None)
    }

    /// Returns the raw string value of a cookie without cryptographic decoding or verification.
    ///
    /// Returns `None` if the cookie does not exist or was deleted in `outgoing`.
    #[must_use]
    pub fn get_plain(&self, name: &str) -> Option<&str> {
        if let Some(cookie) = self.outgoing.get(name) {
            if cookie.is_tombstone() {
                return None;
            }
            return Some(cookie.value());
        }

        self.incoming.get(name).map(String::as_str)
    }

    /// Returns the raw string value of a cookie (alias to `get_plain`).
    #[must_use]
    pub fn get_raw(&self, name: &str) -> Option<&str> {
        self.get_plain(name)
    }

    /// Returns the verified payload of an HMAC-signed cookie.
    ///
    /// Returns `Ok(None)` if the cookie does not exist or was removed.
    /// Returns `Err(CookieError::AuthenticationFailed)` if the cookie exists but is not signed or tampered with.
    pub fn get_signed(&self, name: &str) -> Result<Option<String>, CookieError> {
        let raw = match self.get_plain(name) {
            Some(value) => value,
            None => return Ok(None),
        };

        if detect_security(raw) != CookieSecurity::Signed {
            return Err(CookieError::AuthenticationFailed);
        }

        let keyring = self.keyring.as_ref().ok_or(CookieError::MissingKey)?;
        unpack_signed_with_keyring(name, raw, keyring).map(Some)
    }

    /// Returns the decrypted plaintext of an AEAD-encrypted cookie.
    ///
    /// Returns `Ok(None)` if the cookie does not exist or was removed.
    /// Returns `Err(CookieError::AuthenticationFailed)` if the cookie exists but is not encrypted or tampered with.
    pub fn get_encrypted(&self, name: &str) -> Result<Option<String>, CookieError> {
        let raw = match self.get_plain(name) {
            Some(value) => value,
            None => return Ok(None),
        };

        if detect_security(raw) != CookieSecurity::Encrypted {
            return Err(CookieError::AuthenticationFailed);
        }

        let keyring = self.keyring.as_ref().ok_or(CookieError::MissingKey)?;
        unpack_encrypted_with_keyring(name, raw, keyring).map(Some)
    }

    /// Returns `true` if the cookie exists in incoming or outgoing and has not been tombstoned.
    #[must_use]
    pub fn has(&self, name: &str) -> bool {
        if let Some(cookie) = self.outgoing.get(name) {
            return !cookie.is_tombstone();
        }
        self.incoming.contains_key(name)
    }

    /// Stages an explicitly constructed [`Cookie`] in outgoing response mutations.
    pub fn set(&mut self, cookie: Cookie) {
        self.outgoing.insert(cookie.name().to_string(), cookie);
    }

    /// Stages a plain unencrypted cookie using default jar configuration attributes.
    pub fn set_plain(&mut self, name: &str, value: &str) -> Result<(), CookieError> {
        let cookie = self.create_configured_cookie(name, value)?;
        self.outgoing.insert(name.to_string(), cookie);
        Ok(())
    }

    /// Stages a tamper-proof HMAC-signed cookie using the primary key in the keyring.
    pub fn set_signed(&mut self, name: &str, value: &str) -> Result<(), CookieError> {
        let keyring = self.keyring.as_ref().ok_or(CookieError::MissingKey)?;
        let signer = HmacSigner::new(keyring.primary().clone());
        let packed = pack_signed(name, value, &signer);
        let cookie = self.create_configured_cookie(name, &packed)?;
        self.outgoing.insert(name.to_string(), cookie);
        Ok(())
    }

    /// Stages an AEAD-encrypted cookie using the primary key in the keyring.
    pub fn set_encrypted(&mut self, name: &str, value: &str) -> Result<(), CookieError> {
        let keyring = self.keyring.as_ref().ok_or(CookieError::MissingKey)?;
        let encrypter = Encrypter::new(keyring.clone());
        let packed = pack_encrypted(name, value, &encrypter)?;
        let cookie = self.create_configured_cookie(name, &packed)?;
        self.outgoing.insert(name.to_string(), cookie);
        Ok(())
    }

    /// Stages a deletion tombstone for a cookie using default path and domain.
    pub fn remove(&mut self, name: &str) {
        let default_path = self.config.default_path.clone();
        let default_domain = self.config.default_domain.clone();
        self.remove_with_options(name, Some(&default_path), default_domain.as_deref());
    }

    /// Stages a deletion tombstone with explicit path and domain matching the original cookie.
    pub fn remove_with_options(&mut self, name: &str, path: Option<&str>, domain: Option<&str>) {
        let mut builder = Cookie::build(name, "")
            .max_age(Duration::ZERO)
            .expires(OffsetDateTime::UNIX_EPOCH)
            .secure(self.config.default_secure)
            .http_only(self.config.default_http_only)
            .same_site(self.config.default_same_site);

        if let Some(path_str) = path {
            builder = builder.path(path_str);
        }

        if let Some(domain_str) = domain {
            builder = builder.domain(domain_str);
        }

        if let Ok(cookie) = builder.build() {
            self.outgoing.insert(name.to_string(), cookie);
        }
    }

    /// Unstages and removes a staged cookie from outgoing response mutations without sending a deletion header.
    pub fn unstage(&mut self, name: &str) -> Option<Cookie> {
        self.outgoing.remove(name)
    }

    /// Returns `true` if a cookie has been staged in the outgoing mutation set.
    #[must_use]
    pub fn is_staged(&self, name: &str) -> bool {
        self.outgoing.contains_key(name)
    }

    /// Returns a reference to a staged outgoing cookie if present.
    #[must_use]
    pub fn get_staged(&self, name: &str) -> Option<&Cookie> {
        self.outgoing.get(name)
    }

    /// Returns an iterator over all staged outgoing cookies (added, modified, or deletion tombstones).
    pub fn outgoing(&self) -> impl Iterator<Item = &Cookie> {
        self.outgoing.values()
    }

    /// Returns an iterator over all staged outgoing cookies (alias to `outgoing`).
    pub fn outgoing_cookies(&self) -> impl Iterator<Item = &Cookie> {
        self.outgoing()
    }

    /// Returns a vector of references to all staged outgoing cookies representing the mutation delta.
    #[must_use]
    pub fn delta(&self) -> Vec<&Cookie> {
        self.outgoing.values().collect()
    }

    /// Renders all staged outgoing cookies into formatted `Set-Cookie` header strings.
    #[must_use]
    pub fn render_set_cookie_headers(&self) -> Vec<String> {
        self.outgoing
            .values()
            .map(Cookie::to_set_cookie_header)
            .collect()
    }

    /// Returns an iterator over incoming request cookies parsed from request headers.
    pub fn incoming(&self) -> impl Iterator<Item = (&str, &str)> {
        self.incoming
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Returns the number of incoming cookies.
    #[must_use]
    pub fn incoming_count(&self) -> usize {
        self.incoming.len()
    }

    /// Returns the number of staged outgoing cookies.
    #[must_use]
    pub fn outgoing_count(&self) -> usize {
        self.outgoing.len()
    }

    /// Returns `true` if there are neither incoming nor outgoing cookies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.incoming.is_empty() && self.outgoing.is_empty()
    }

    fn decode_value(&self, name: &str, raw: &str) -> Result<String, CookieError> {
        match detect_security(raw) {
            CookieSecurity::Plain => Ok(raw.to_string()),
            CookieSecurity::Signed => {
                let keyring = self.keyring.as_ref().ok_or(CookieError::MissingKey)?;
                unpack_signed_with_keyring(name, raw, keyring)
            }
            CookieSecurity::Encrypted => {
                let keyring = self.keyring.as_ref().ok_or(CookieError::MissingKey)?;
                unpack_encrypted_with_keyring(name, raw, keyring)
            }
        }
    }

    fn create_configured_cookie(&self, name: &str, value: &str) -> Result<Cookie, CookieError> {
        let mut builder = Cookie::build(name, value)
            .path(self.config.default_path.clone())
            .http_only(self.config.default_http_only)
            .secure(self.config.default_secure)
            .partitioned(self.config.default_partitioned);

        if let Some(domain) = &self.config.default_domain {
            builder = builder.domain(domain.clone());
        }

        if let Some(same_site) = self.config.default_same_site {
            builder = builder.same_site(same_site);
        }

        if let Some(max_age) = self.config.default_max_age {
            builder = builder.max_age(max_age);
        }

        builder.build()
    }
}
