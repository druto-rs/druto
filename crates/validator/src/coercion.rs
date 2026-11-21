//! Type coercion, sanitization, and normalization for incoming untrusted payloads.

use serde_json::{Number, Value};

/// Sanitizes a string slice by trimming leading and trailing whitespace.
#[must_use]
pub fn trim_whitespace(input: &str) -> &str {
    input.trim()
}

/// Normalizes empty string values `""` into JSON `Null`.
///
/// Critical for HTML form submissions where omitted text fields are submitted as empty strings
/// rather than null or missing keys.
#[must_use]
pub fn empty_string_to_null(value: &Value) -> Value {
    match value {
        Value::String(string_val) if string_val.trim().is_empty() => Value::Null,
        other => other.clone(),
    }
}

/// Recursively traverses a JSON value, normalizing empty strings `""` to `Null`.
#[must_use]
pub fn normalize_empty_strings(value: &Value) -> Value {
    match value {
        Value::String(string_val) if string_val.trim().is_empty() => Value::Null,
        Value::Array(items) => Value::Array(items.iter().map(normalize_empty_strings).collect()),
        Value::Object(map) => {
            let normalized = map
                .iter()
                .map(|(key, val)| (key.clone(), normalize_empty_strings(val)))
                .collect();
            Value::Object(normalized)
        }
        other => other.clone(),
    }
}

/// Attempts to coerce a JSON value into a `Number`.
///
/// Converts numeric strings (such as `"42"` or `"3.14"`) from query strings or forms into numbers.
#[must_use]
pub fn coerce_to_number(value: &Value) -> Option<Number> {
    match value {
        Value::Number(num) => Some(num.clone()),
        Value::String(str_val) => {
            let trimmed = str_val.trim();
            if let Ok(int_val) = trimmed.parse::<i64>() {
                Some(Number::from(int_val))
            } else if let Ok(uint_val) = trimmed.parse::<u64>() {
                Some(Number::from(uint_val))
            } else if let Ok(float_val) = trimmed.parse::<f64>() {
                Number::from_f64(float_val)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Attempts to coerce a JSON value into a boolean.
///
/// Converts truthy strings (`"true"`, `"1"`, `"yes"`, `"on"`) and falsy strings (`"false"`, `"0"`, `"no"`, `"off"`).
#[must_use]
pub fn coerce_to_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(boolean_val) => Some(*boolean_val),
        Value::Number(num) => {
            if let Some(int_val) = num.as_i64() {
                match int_val {
                    1 => Some(true),
                    0 => Some(false),
                    _ => None,
                }
            } else {
                None
            }
        }
        Value::String(str_val) => {
            let normalized = str_val.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Attempts to coerce a JSON value into an owned string.
#[must_use]
pub fn coerce_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(str_val) => Some(str_val.clone()),
        Value::Number(num) => Some(num.to_string()),
        Value::Bool(boolean_val) => Some(boolean_val.to_string()),
        _ => None,
    }
}

/// Ensures an array value is produced, wrapping a single scalar if necessary.
#[must_use]
pub fn coerce_to_array(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::Array(items) => Some(items.clone()),
        Value::Null => None,
        scalar => Some(vec![scalar.clone()]),
    }
}
