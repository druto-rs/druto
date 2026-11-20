//! JSON request payload parser with size and nesting depth bounds.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{error::BodyParserError, limits::PayloadLimits};

/// Parses raw bytes as JSON with strict size and nesting depth limits.
pub fn parse_json<T: DeserializeOwned>(
    bytes: &[u8],
    limits: &PayloadLimits,
) -> Result<T, BodyParserError> {
    limits.check_body_size(bytes.len())?;

    let value: Value = serde_json::from_slice(bytes)?;
    verify_depth(&value, 1, limits.max_depth())?;

    serde_json::from_value(value).map_err(BodyParserError::from)
}

/// Parses raw bytes into a generic [`Value`] with limits applied.
pub fn parse_json_value(bytes: &[u8], limits: &PayloadLimits) -> Result<Value, BodyParserError> {
    limits.check_body_size(bytes.len())?;

    let value: Value = serde_json::from_slice(bytes)?;
    verify_depth(&value, 1, limits.max_depth())?;

    Ok(value)
}

/// Recursively verifies container nesting depth does not exceed `max_depth`.
fn verify_depth(
    value: &Value,
    current_depth: usize,
    max_depth: usize,
) -> Result<(), BodyParserError> {
    if current_depth > max_depth {
        return Err(BodyParserError::NestingDepthExceeded { max_depth });
    }

    match value {
        Value::Array(items) => {
            for item in items {
                verify_depth(item, current_depth + 1, max_depth)?;
            }
        }
        Value::Object(map) => {
            for val in map.values() {
                verify_depth(val, current_depth + 1, max_depth)?;
            }
        }
        _ => {}
    }

    Ok(())
}
