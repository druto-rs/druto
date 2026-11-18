//! Nested query string parser and serializer supporting deep object and array syntax.
//!
//! # Overview
//!
//! `qs` provides nested, bracket-notation query string parsing and serialization for the Druto
//! framework. It enables structured query parameters sent by modern frontends (`a[b]=c`, `ids[]=1`,
//! `users[0][name]=John`) to be parsed into dynamic [`QsValue`] trees or deserialized directly
//! into strongly typed Rust structs via [`serde`].
//!
//! # Features
//!
//! - **Nested Bracket & Dot Notation**: Handles multi-level associative objects, sequential arrays,
//!   and indexed arrays.
//! - **Defensive Resource Limits**: Protects against DoS via recursion depth limits, parameter count
//!   limits, and sparse array allocation bombs.
//! - **Configurable Serialization**: Supports multiple array formats (`Indices`, `Brackets`, `Repeat`,
//!   `Comma`), deterministic key sorting, and RFC 3986 percent encoding.
//! - **First-Class Serde Integration**: Permissive deserialization that automatically coerces
//!   string-encoded numbers and booleans into Rust primitive types.

pub mod de;
pub mod encoding;
pub mod error;
pub mod options;
pub mod parser;
pub mod ser;
pub mod stringify;
pub mod value;

pub use de::{from_str, from_str_with_options};
pub use error::QsError;
pub use options::{ArrayFormat, Duplicates, ParseOptions, StringifyOptions};
pub use parser::parse_query;
pub use ser::{to_string, to_string_with_options, to_value};
pub use stringify::stringify_query;
pub use value::QsValue;

/// Parses a query string into a [`QsValue`] tree using default [`ParseOptions`].
///
/// # Errors
///
/// Returns a [`QsError`] if parsing limits are exceeded or syntax is invalid.
pub fn parse(query: &str) -> Result<QsValue, QsError> {
    parse_query(query, &ParseOptions::default())
}

/// Parses a query string into a [`QsValue`] tree using custom [`ParseOptions`].
///
/// # Errors
///
/// Returns a [`QsError`] if parsing limits are exceeded or syntax is invalid.
pub fn parse_with_options(query: &str, options: &ParseOptions) -> Result<QsValue, QsError> {
    parse_query(query, options)
}

/// Serializes a [`QsValue`] tree into an RFC 3986 query string using default [`StringifyOptions`].
#[must_use]
pub fn stringify(value: &QsValue) -> String {
    stringify_query(value, &StringifyOptions::default())
}

/// Serializes a [`QsValue`] tree into an RFC 3986 query string using custom [`StringifyOptions`].
#[must_use]
pub fn stringify_with_options(value: &QsValue, options: &StringifyOptions) -> String {
    stringify_query(value, options)
}
