//! RFC 6265 cookie model, builder, and header parser/serializer.

use std::fmt;

use time::{Duration, OffsetDateTime, UtcOffset, format_description::well_known::Rfc2822};

use crate::{error::CookieError, same_site::SameSite};

const IMF_DATE_FORMAT: &str =
    "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT";

/// An RFC 6265 compliant HTTP cookie with all standardized attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    name: String,
    value: String,
    path: Option<String>,
    domain: Option<String>,
    expires: Option<OffsetDateTime>,
    max_age: Option<Duration>,
    secure: bool,
    http_only: bool,
    same_site: Option<SameSite>,
    partitioned: bool,
}

impl Cookie {
    /// Creates a new cookie with the given name and value, validating token syntax.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Result<Self, CookieError> {
        let name_str = name.into();
        let value_str = value.into();

        Self::validate_name(&name_str)?;
        Self::validate_value(&value_str)?;

        Ok(Self {
            name: name_str,
            value: value_str,
            path: None,
            domain: None,
            expires: None,
            max_age: None,
            secure: false,
            http_only: false,
            same_site: None,
            partitioned: false,
        })
    }

    /// Creates a fluent builder for configuring a new cookie.
    pub fn build(name: impl Into<String>, value: impl Into<String>) -> CookieBuilder {
        CookieBuilder::new(name, value)
    }

    /// Returns the cookie name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the cookie value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Sets the cookie value, validating token syntax.
    pub fn set_value(&mut self, value: impl Into<String>) -> Result<(), CookieError> {
        let value_str = value.into();
        Self::validate_value(&value_str)?;
        self.value = value_str;
        Ok(())
    }

    /// Returns the cookie `Path` attribute.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Sets the cookie `Path` attribute.
    pub fn set_path(&mut self, path: impl Into<String>) {
        self.path = Some(path.into());
    }

    /// Returns the cookie `Domain` attribute.
    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// Sets the cookie `Domain` attribute.
    pub fn set_domain(&mut self, domain: impl Into<String>) {
        self.domain = Some(domain.into());
    }

    /// Returns the cookie `Expires` attribute.
    #[must_use]
    pub const fn expires(&self) -> Option<OffsetDateTime> {
        self.expires
    }

    /// Sets the cookie `Expires` attribute.
    pub fn set_expires(&mut self, expires: impl Into<Option<OffsetDateTime>>) {
        self.expires = expires.into();
    }

    /// Returns the cookie `Max-Age` attribute.
    #[must_use]
    pub const fn max_age(&self) -> Option<Duration> {
        self.max_age
    }

    /// Sets the cookie `Max-Age` attribute.
    pub fn set_max_age(&mut self, max_age: impl Into<Option<Duration>>) {
        self.max_age = max_age.into();
    }

    /// Returns the cookie `Secure` flag.
    #[must_use]
    pub const fn secure(&self) -> bool {
        self.secure
    }

    /// Sets the cookie `Secure` flag.
    pub fn set_secure(&mut self, secure: bool) {
        self.secure = secure;
    }

    /// Returns the cookie `HttpOnly` flag.
    #[must_use]
    pub const fn http_only(&self) -> bool {
        self.http_only
    }

    /// Sets the cookie `HttpOnly` flag.
    pub fn set_http_only(&mut self, http_only: bool) {
        self.http_only = http_only;
    }

    /// Returns the cookie `SameSite` attribute.
    #[must_use]
    pub const fn same_site(&self) -> Option<SameSite> {
        self.same_site
    }

    /// Sets the cookie `SameSite` attribute.
    pub fn set_same_site(&mut self, same_site: impl Into<Option<SameSite>>) {
        self.same_site = same_site.into();
    }

    /// Returns whether the cookie uses the CHIPS `Partitioned` attribute.
    #[must_use]
    pub const fn partitioned(&self) -> bool {
        self.partitioned
    }

    /// Sets the CHIPS `Partitioned` attribute.
    pub fn set_partitioned(&mut self, partitioned: bool) {
        self.partitioned = partitioned;
    }

    /// Returns `true` if this cookie represents a deletion tombstone (e.g. `Max-Age=0` or expired timestamp).
    #[must_use]
    pub fn is_tombstone(&self) -> bool {
        self.max_age
            .is_some_and(|max_age| max_age.whole_seconds() <= 0)
            || self
                .expires
                .is_some_and(|expires| expires.unix_timestamp() <= 0)
    }

    /// Serializes the cookie into a standard `Set-Cookie` header value.
    #[must_use]
    pub fn to_set_cookie_header(&self) -> String {
        let mut parts = Vec::with_capacity(8);
        parts.push(format!("{}={}", self.name, self.value));

        if let Some(path) = &self.path {
            parts.push(format!("Path={path}"));
        }

        if let Some(domain) = &self.domain {
            parts.push(format!("Domain={domain}"));
        }

        if let Some(formatted) = self.expires.and_then(|dt| Self::format_date(dt).ok()) {
            parts.push(format!("Expires={formatted}"));
        }

        if let Some(max_age) = self.max_age {
            parts.push(format!("Max-Age={}", max_age.whole_seconds()));
        }

        if self.secure {
            parts.push("Secure".to_string());
        }

        if self.http_only {
            parts.push("HttpOnly".to_string());
        }

        if let Some(same_site) = self.same_site {
            parts.push(format!("SameSite={same_site}"));
        }

        if self.partitioned {
            parts.push("Partitioned".to_string());
        }

        parts.join("; ")
    }

    /// Parses an incoming client `Cookie` header string into name-value pairs tolerantly.
    #[must_use]
    pub fn parse_cookie_header(header: &str) -> Vec<(String, String)> {
        let mut pairs = Vec::new();

        for entry in header.split(';') {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some((name, value)) = trimmed.split_once('=') {
                let clean_name = name.trim();
                let clean_value = value.trim();

                let stripped_value = strip_quotes(clean_value);

                if Self::validate_name(clean_name).is_ok() {
                    pairs.push((clean_name.to_string(), stripped_value.to_string()));
                }
            }
        }

        pairs
    }

    /// Parses a single `Set-Cookie` header value into a structured [`Cookie`].
    pub fn parse_set_cookie(header: &str) -> Result<Self, CookieError> {
        let mut parts = header.split(';');

        let first = parts.next().unwrap_or_default().trim();
        if first.is_empty() {
            return Err(CookieError::MalformedHeader(
                "empty Set-Cookie header".to_string(),
            ));
        }

        let (name, value) = first.split_once('=').ok_or_else(|| {
            CookieError::MalformedHeader(format!("missing '=' in Set-Cookie: {first}"))
        })?;

        let clean_val = strip_quotes(value.trim());
        let mut cookie = Cookie::new(name.trim(), clean_val)?;

        for attr in parts {
            let trimmed = attr.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some((attr_name, attr_val)) = trimmed.split_once('=') {
                let lower_attr = attr_name.trim().to_ascii_lowercase();
                let val = attr_val.trim();

                match lower_attr.as_str() {
                    "path" => cookie.set_path(val),
                    "domain" => cookie.set_domain(val),
                    "max-age" => {
                        let seconds = val.parse::<i64>().map_err(|_| {
                            CookieError::MalformedHeader(format!("invalid Max-Age: {val}"))
                        })?;
                        cookie.set_max_age(Duration::seconds(seconds));
                    }
                    "expires" => {
                        let parsed_date = Self::parse_date(val)?;
                        cookie.set_expires(parsed_date);
                    }
                    "samesite" => {
                        let same_site = SameSite::parse(val)?;
                        cookie.set_same_site(same_site);
                    }
                    _ => {}
                }
            } else {
                let lower_flag = trimmed.to_ascii_lowercase();
                match lower_flag.as_str() {
                    "secure" => cookie.set_secure(true),
                    "httponly" => cookie.set_http_only(true),
                    "partitioned" => cookie.set_partitioned(true),
                    _ => {}
                }
            }
        }

        Ok(cookie)
    }

    fn validate_name(name: &str) -> Result<(), CookieError> {
        if name.is_empty() {
            return Err(CookieError::InvalidName(
                "cookie name cannot be empty".to_string(),
            ));
        }

        for byte in name.bytes() {
            // RFC 6265 token characters (US-ASCII excluding control chars and separators)
            let is_valid = !matches!(
                byte,
                0..=31
                    | 127..=255
                    | b'('
                    | b')'
                    | b'<'
                    | b'>'
                    | b'@'
                    | b','
                    | b';'
                    | b':'
                    | b'\\'
                    | b'"'
                    | b'/'
                    | b'['
                    | b']'
                    | b'?'
                    | b'='
                    | b'{'
                    | b'}'
                    | b' '
            );

            if !is_valid {
                return Err(CookieError::InvalidName(format!(
                    "invalid character in cookie name: '{}'",
                    byte as char
                )));
            }
        }

        Ok(())
    }

    fn validate_value(value: &str) -> Result<(), CookieError> {
        for byte in value.bytes() {
            // RFC 6265 cookie-octet (ASCII 33-126 excluding comma, semicolon, and backslash)
            let is_valid = !matches!(byte, 0..=31 | 127..=255 | b' ' | b',' | b';' | b'\\');

            if !is_valid {
                return Err(CookieError::InvalidValue(format!(
                    "invalid character in cookie value: '{}'",
                    byte as char
                )));
            }
        }

        Ok(())
    }

    fn format_date(dt: OffsetDateTime) -> Result<String, CookieError> {
        let items = time::format_description::parse_borrowed::<2>(IMF_DATE_FORMAT)
            .map_err(|err| CookieError::EncodingError(err.to_string()))?;
        dt.to_offset(UtcOffset::UTC)
            .format(&items)
            .map_err(|err| CookieError::EncodingError(err.to_string()))
    }

    fn parse_date(date_str: &str) -> Result<OffsetDateTime, CookieError> {
        let items = time::format_description::parse_borrowed::<2>(IMF_DATE_FORMAT)
            .map_err(|err| CookieError::EncodingError(err.to_string()))?;

        OffsetDateTime::parse(date_str, &items)
            .or_else(|_| OffsetDateTime::parse(date_str, &Rfc2822))
            .map_err(|err| CookieError::MalformedHeader(format!("invalid cookie date: {err}")))
    }
}

#[inline]
fn strip_quotes(value: &str) -> &str {
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

impl fmt::Display for Cookie {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_set_cookie_header())
    }
}

/// Fluent builder for constructing a [`Cookie`] with custom attributes.
#[derive(Debug, Clone)]
pub struct CookieBuilder {
    inner: Cookie,
}

impl CookieBuilder {
    /// Creates a new builder instance.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            inner: Cookie {
                name: name.into(),
                value: value.into(),
                path: None,
                domain: None,
                expires: None,
                max_age: None,
                secure: false,
                http_only: false,
                same_site: None,
                partitioned: false,
            },
        }
    }

    /// Sets the cookie path attribute.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.inner.path = Some(path.into());
        self
    }

    /// Sets the cookie domain attribute.
    #[must_use]
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.inner.domain = Some(domain.into());
        self
    }

    /// Sets the cookie expiration timestamp.
    #[must_use]
    pub fn expires(mut self, expires: impl Into<Option<OffsetDateTime>>) -> Self {
        self.inner.expires = expires.into();
        self
    }

    /// Sets the cookie max-age duration.
    #[must_use]
    pub fn max_age(mut self, max_age: impl Into<Option<Duration>>) -> Self {
        self.inner.max_age = max_age.into();
        self
    }

    /// Sets the cookie secure flag.
    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.inner.secure = secure;
        self
    }

    /// Sets the cookie HttpOnly flag.
    #[must_use]
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.inner.http_only = http_only;
        self
    }

    /// Sets the cookie SameSite policy.
    #[must_use]
    pub fn same_site(mut self, same_site: impl Into<Option<SameSite>>) -> Self {
        self.inner.same_site = same_site.into();
        self
    }

    /// Sets the cookie Partitioned flag (CHIPS).
    #[must_use]
    pub fn partitioned(mut self, partitioned: bool) -> Self {
        self.inner.partitioned = partitioned;
        self
    }

    /// Builds the validated [`Cookie`].
    pub fn build(self) -> Result<Cookie, CookieError> {
        Cookie::validate_name(&self.inner.name)?;
        Cookie::validate_value(&self.inner.value)?;
        Ok(self.inner)
    }
}
