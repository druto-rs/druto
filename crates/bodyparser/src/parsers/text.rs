//! Raw text request payload parser.

use crate::{error::BodyParserError, limits::PayloadLimits};

/// Parses raw payload bytes into a validated UTF-8 string with size bounds.
pub fn parse_text(bytes: &[u8], limits: &PayloadLimits) -> Result<String, BodyParserError> {
    limits.check_body_size(bytes.len())?;
    let text = std::str::from_utf8(bytes)?;
    Ok(text.to_string())
}
