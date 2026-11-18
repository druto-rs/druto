//! Percent-encoding and decoding compliant with RFC 3986.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC};

/// RFC 3986 unreserved character set: `ALPHA / DIGIT / "-" / "." / "_" / "~"`.
pub const RFC3986: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// RFC 3986 character set for keys that preserves `[` and `]` brackets.
pub const RFC3986_KEY_WITH_BRACKETS: &AsciiSet = &RFC3986.remove(b'[').remove(b']');

/// Decodes a percent-encoded query string component.
///
/// In query strings, literal `+` characters represent spaces, whereas encoded
/// `%2B` representations decode to `+`.
#[must_use]
pub fn decode_component(raw: &str) -> String {
    if !raw.contains('+') && !raw.contains('%') {
        return raw.to_string();
    }

    let replaced = raw.replace('+', " ");
    percent_encoding::percent_decode_str(&replaced)
        .decode_utf8_lossy()
        .into_owned()
}

/// Percent-encodes a value using the RFC 3986 unreserved character set.
#[must_use]
pub fn encode_component(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, RFC3986).to_string()
}

/// Percent-encodes a key while keeping structural brackets `[` and `]` intact.
#[must_use]
pub fn encode_key_preserving_brackets(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, RFC3986_KEY_WITH_BRACKETS).to_string()
}
