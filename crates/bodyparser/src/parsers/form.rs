//! URL-encoded form request payload parser with nested bracket notation and bounds.

use khonj::{Value, options::ParseOptions};
use serde::de::DeserializeOwned;

use crate::{error::BodyParserError, limits::PayloadLimits};

/// Parses URL-encoded form bytes into a typed struct using bracket-notation parsing.
pub fn parse_form<T: DeserializeOwned>(
    bytes: &[u8],
    limits: &PayloadLimits,
) -> Result<T, BodyParserError> {
    limits.check_body_size(bytes.len())?;

    let text = std::str::from_utf8(bytes)?;

    let options = ParseOptions::new()
        .depth(limits.max_depth())
        .parameter_limit(limits.max_fields_count());

    khonj::from_str_with_options::<T>(text, &options).map_err(BodyParserError::from)
}

/// Parses URL-encoded form bytes into a structured [`Value`] tree.
pub fn parse_form_value(bytes: &[u8], limits: &PayloadLimits) -> Result<Value, BodyParserError> {
    limits.check_body_size(bytes.len())?;

    let text = std::str::from_utf8(bytes)?;

    let options = ParseOptions::new()
        .depth(limits.max_depth())
        .parameter_limit(limits.max_fields_count());

    khonj::parse_with_options(text, &options).map_err(BodyParserError::from)
}
