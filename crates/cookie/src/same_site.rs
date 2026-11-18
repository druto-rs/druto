//! RFC 6265bis SameSite attribute values.

use std::fmt;

use crate::error::CookieError;

/// The `SameSite` attribute controls whether a cookie is sent with cross-site requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SameSite {
    /// Cookies are only sent in a first-party context.
    Strict,
    /// Cookies are withheld on cross-site subrequests, but sent when a user navigates to the origin site.
    #[default]
    Lax,
    /// Cookies will be sent in all contexts, including cross-site requests (requires `Secure`).
    None,
}

impl SameSite {
    /// Returns the standard RFC 6265 string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Strict => "Strict",
            Self::Lax => "Lax",
            Self::None => "None",
        }
    }

    /// Parses a `SameSite` attribute value case-insensitively.
    pub fn parse(value: &str) -> Result<Self, CookieError> {
        let trimmed = value.trim();
        if trimmed.eq_ignore_ascii_case("strict") {
            Ok(Self::Strict)
        } else if trimmed.eq_ignore_ascii_case("lax") {
            Ok(Self::Lax)
        } else if trimmed.eq_ignore_ascii_case("none") {
            Ok(Self::None)
        } else {
            Err(CookieError::MalformedHeader(format!(
                "invalid SameSite attribute value: '{value}'"
            )))
        }
    }
}

impl fmt::Display for SameSite {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
