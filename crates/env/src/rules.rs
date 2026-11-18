use std::{any::Any, collections::HashMap, fmt, str::FromStr, time::Duration};

use crate::{error::ValidationError, secret::Secret};

/// Internal trait implemented by all validation rules to produce a boxed typed value.
pub trait FieldValidator: Send + Sync {
    /// Validates a single key from the raw environment map.
    fn validate(
        &self,
        key: &str,
        raw_map: &HashMap<String, String>,
    ) -> Result<Box<dyn Any + Send + Sync>, ValidationError>;
}

/// Type alias for rule validation closures.
pub type RuleValidator<T> =
    Box<dyn Fn(&str, &HashMap<String, String>) -> Result<T, ValidationError> + Send + Sync>;

/// A validation rule for an environment variable producing a strongly-typed value `T`.
pub struct Rule<T> {
    validator: RuleValidator<T>,
}

impl<T: Send + Sync + 'static> FieldValidator for Rule<T> {
    fn validate(
        &self,
        key: &str,
        raw_map: &HashMap<String, String>,
    ) -> Result<Box<dyn Any + Send + Sync>, ValidationError> {
        let value = (self.validator)(key, raw_map)?;
        Ok(Box::new(value))
    }
}

impl<T: Send + Sync + Clone + 'static> Rule<T> {
    /// Provides a fallback default value if the environment variable is missing or empty.
    pub fn default_value(self, default: T) -> Rule<T> {
        Rule {
            validator: Box::new(move |key, raw_map| match (self.validator)(key, raw_map) {
                Ok(val) => Ok(val),
                Err(err) => {
                    let is_missing = raw_map.get(key).is_none_or(|val| val.trim().is_empty());
                    if is_missing {
                        Ok(default.clone())
                    } else {
                        Err(err)
                    }
                }
            }),
        }
    }

    /// Makes the environment variable optional, producing an `Option<T>`.
    pub fn optional(self) -> Rule<Option<T>> {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let is_missing = raw_map.get(key).is_none_or(|val| val.trim().is_empty());
                if is_missing {
                    Ok(None)
                } else {
                    let parsed = (self.validator)(key, raw_map)?;
                    Ok(Some(parsed))
                }
            }),
        }
    }
}

impl Rule<String> {
    /// Validates that the variable is a string.
    pub fn string() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) if !value.is_empty() => Ok(value.clone()),
                _ => Err(ValidationError::new(key, "is required and cannot be empty")),
            }),
        }
    }

    /// Requires the string to not be empty.
    pub fn not_empty(self) -> Self {
        self.min_length(1)
    }

    /// Requires the string to have at least `min` characters.
    pub fn min_length(self, min: usize) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if value.len() < min {
                    Err(ValidationError::new(
                        key,
                        format!("must have at least {min} characters"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }

    /// Requires the string to have at most `max` characters.
    pub fn max_length(self, max: usize) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if value.len() > max {
                    Err(ValidationError::new(
                        key,
                        format!("must have at most {max} characters"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }

    /// Requires the string to start with the specified prefix.
    pub fn starts_with(self, prefix: &'static str) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if !value.starts_with(prefix) {
                    Err(ValidationError::new(
                        key,
                        format!("must start with `{prefix}`"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }

    /// Requires the string to end with the specified suffix.
    pub fn ends_with(self, suffix: &'static str) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if !value.ends_with(suffix) {
                    Err(ValidationError::new(
                        key,
                        format!("must end with `{suffix}`"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }

    /// Matches the string against a regular expression pattern.
    #[cfg(feature = "regex")]
    pub fn regex(self, pattern: &'static str) -> Self {
        let compiled = regex::Regex::new(pattern).expect("invalid regular expression pattern");
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if !compiled.is_match(&value) {
                    Err(ValidationError::new(
                        key,
                        format!("does not match required pattern `{pattern}`"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }
}

impl Rule<bool> {
    /// Validates and parses boolean values (`true`, `false`, `1`, `0`, `yes`, `no`, `on`, `off`).
    pub fn boolean() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => {
                    let normalized = value.trim().to_ascii_lowercase();
                    match normalized.as_str() {
                        "true" | "1" | "yes" | "on" => Ok(true),
                        "false" | "0" | "no" | "off" => Ok(false),
                        _ => Err(ValidationError::new(
                            key,
                            "must be a valid boolean (true/false, 1/0, yes/no, on/off)",
                        )),
                    }
                }
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }
}

impl Rule<u16> {
    /// Validates that the variable is a valid network port number (1..=65535).
    pub fn port() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => match value.trim().parse::<u16>() {
                    Ok(port_number) if port_number > 0 => Ok(port_number),
                    _ => Err(ValidationError::new(
                        key,
                        "must be a valid network port between 1 and 65535",
                    )),
                },
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }
}

impl Rule<String> {
    /// Validates that the variable is a well-formed URL.
    pub fn url() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => {
                    let trimmed = value.trim();
                    let separator = trimmed.find("://");
                    let has_valid_scheme = separator.is_some_and(|pos| {
                        pos > 0
                            && trimmed[..pos].chars().all(|ch| {
                                ch.is_ascii_alphanumeric() || ch == '+' || ch == '-' || ch == '.'
                            })
                    });
                    let has_host = separator.is_some_and(|pos| {
                        let rest = &trimmed[pos + 3..];
                        !rest.is_empty()
                    });

                    if has_valid_scheme && has_host {
                        Ok(trimmed.to_string())
                    } else {
                        Err(ValidationError::new(
                            key,
                            "must be a valid URL with scheme and host",
                        ))
                    }
                }
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }

    /// Enforces that the string value is one of the allowed enum options.
    pub fn enum_values(allowed: &'static [&'static str]) -> Self {
        Self {
            validator: Box::new(move |key, raw_map| match raw_map.get(key) {
                Some(value) => {
                    let trimmed = value.trim();
                    if allowed.contains(&trimmed) {
                        Ok(trimmed.to_string())
                    } else {
                        Err(ValidationError::new(
                            key,
                            format!("must be one of: {}", allowed.join(", ")),
                        ))
                    }
                }
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }
}

impl Rule<Duration> {
    /// Validates and parses time duration strings (e.g. `500ms`, `10s`, `15m`, `2h`, `1d`).
    pub fn duration() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => parse_duration(value.trim())
                    .map_err(|message| ValidationError::new(key, message)),
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }
}

impl Rule<u64> {
    /// Validates and parses byte sizes (e.g. `512B`, `10KB`, `50MB`, `2GB`).
    pub fn bytes() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => {
                    parse_bytes(value.trim()).map_err(|message| ValidationError::new(key, message))
                }
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }
}

impl<T: FromStr + PartialOrd + fmt::Display + Send + Sync + 'static> Rule<T>
where
    T::Err: fmt::Display,
{
    /// Validates and parses an integer or numeric type.
    pub fn integer() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => value
                    .trim()
                    .parse::<T>()
                    .map_err(|_| ValidationError::new(key, "must be a valid integer")),
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }

    /// Validates and parses a floating-point number.
    pub fn float() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => value
                    .trim()
                    .parse::<T>()
                    .map_err(|_| ValidationError::new(key, "must be a valid float")),
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }

    /// Enforces a minimum value.
    pub fn min(self, minimum: T) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if value < minimum {
                    Err(ValidationError::new(
                        key,
                        format!("must be at least {minimum}"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }

    /// Enforces a maximum value.
    pub fn max(self, maximum: T) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if value > maximum {
                    Err(ValidationError::new(
                        key,
                        format!("must be at most {maximum}"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }

    /// Enforces an inclusive range between `minimum` and `maximum`.
    pub fn range(self, minimum: T, maximum: T) -> Self {
        Rule {
            validator: Box::new(move |key, raw_map| {
                let value = (self.validator)(key, raw_map)?;
                if value < minimum || value > maximum {
                    Err(ValidationError::new(
                        key,
                        format!("must be between {minimum} and {maximum}"),
                    ))
                } else {
                    Ok(value)
                }
            }),
        }
    }
}

impl<T: Send + Sync + 'static> Rule<Secret<T>> {
    /// Wraps an inner rule's validated value in a `Secret<T>` to prevent leakage in debug and logs.
    pub fn secret(inner_rule: Rule<T>) -> Self {
        Self {
            validator: Box::new(move |key, raw_map| {
                let value = (inner_rule.validator)(key, raw_map)?;
                Ok(Secret::new(value))
            }),
        }
    }
}

impl<T: FromStr + Send + Sync + 'static> Rule<Vec<T>>
where
    T::Err: fmt::Display,
{
    /// Parses a comma-separated list of items into a `Vec<T>`.
    pub fn list() -> Self {
        Self {
            validator: Box::new(|key, raw_map| match raw_map.get(key) {
                Some(value) => {
                    let mut items = Vec::new();
                    for part in value.split(',') {
                        let trimmed = part.trim();
                        if !trimmed.is_empty() {
                            match trimmed.parse::<T>() {
                                Ok(item) => items.push(item),
                                Err(_) => {
                                    return Err(ValidationError::new(
                                        key,
                                        "contains invalid list item",
                                    ));
                                }
                            }
                        }
                    }
                    Ok(items)
                }
                None => Err(ValidationError::new(key, "is required")),
            }),
        }
    }
}

fn split_number_prefix(input: &str) -> (&str, &str) {
    let mut split_pos = 0;
    for (idx, ch) in input.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || (idx == 0 && ch == '-') {
            split_pos = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    (&input[..split_pos], input[split_pos..].trim())
}

fn parse_duration(input: &str) -> Result<Duration, String> {
    if input.is_empty() {
        return Err("duration cannot be empty".to_string());
    }

    let (number_str, unit_str) = split_number_prefix(input);
    if number_str.is_empty() {
        return Err("missing numeric value in duration".to_string());
    }

    let unit = unit_str.to_ascii_lowercase();

    let amount = number_str
        .parse::<f64>()
        .map_err(|_| "invalid number in duration".to_string())?;

    if amount < 0.0 || amount.is_nan() {
        return Err("duration cannot be negative".to_string());
    }

    match unit.as_str() {
        "ms" | "millis" | "milliseconds" => {
            let millis = amount.round();
            if millis > u64::MAX as f64 {
                return Err("duration value is too large".to_string());
            }
            Ok(Duration::from_millis(millis as u64))
        }
        "s" | "sec" | "secs" | "seconds" => Duration::try_from_secs_f64(amount)
            .map_err(|_| "duration value is too large".to_string()),
        "m" | "min" | "mins" | "minutes" => Duration::try_from_secs_f64(amount * 60.0)
            .map_err(|_| "duration value is too large".to_string()),
        "h" | "hr" | "hrs" | "hours" => Duration::try_from_secs_f64(amount * 3600.0)
            .map_err(|_| "duration value is too large".to_string()),
        "d" | "day" | "days" => Duration::try_from_secs_f64(amount * 86400.0)
            .map_err(|_| "duration value is too large".to_string()),
        _ => Err(format!(
            "unknown duration unit `{unit}` (use ms, s, m, h, d)"
        )),
    }
}

fn parse_bytes(input: &str) -> Result<u64, String> {
    if input.is_empty() {
        return Err("byte size cannot be empty".to_string());
    }

    let (number_str, unit_str) = split_number_prefix(input);
    if number_str.is_empty() {
        return Err("missing numeric value in byte size".to_string());
    }

    let unit = unit_str.to_ascii_lowercase();

    let amount = number_str
        .parse::<f64>()
        .map_err(|_| "invalid number in byte size".to_string())?;

    if amount < 0.0 || amount.is_nan() {
        return Err("byte size cannot be negative".to_string());
    }

    let multiplier: u64 = match unit.as_str() {
        "" | "b" | "bytes" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024 * 1024 * 1024 * 1024,
        _ => {
            return Err(format!(
                "unknown byte unit `{unit}` (use b, kb, mb, gb, tb)"
            ));
        }
    };

    let total = amount * (multiplier as f64);
    if total > u64::MAX as f64 {
        return Err("byte size value is too large".to_string());
    }

    Ok(total as u64)
}
