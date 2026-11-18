//! Parameter validation constraints including high-performance built-in checks and regular expressions.

use std::{fmt, sync::Arc};

use crate::error::MatcherError;

/// Built-in constraint shortcuts providing zero-allocation ASCII validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinConstraint {
    /// Matches non-empty ASCII digit strings (`^[0-9]+$`).
    Num,
    /// Matches standard 36-character hexadecimal UUIDs (`8-4-4-4-12`).
    Uuid,
    /// Matches URL slug tokens (alphanumeric, hyphens, and underscores).
    Slug,
    /// Matches non-empty ASCII letters (`^[a-zA-Z]+$`).
    Alpha,
    /// Matches non-empty ASCII letters and digits (`^[a-zA-Z0-9]+$`).
    Alphanumeric,
}

impl BuiltinConstraint {
    /// Validates whether the given value satisfies the built-in constraint.
    #[must_use]
    pub fn matches(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        match self {
            Self::Num => value.chars().all(|ch| ch.is_ascii_digit()),
            Self::Alpha => value.chars().all(|ch| ch.is_ascii_alphabetic()),
            Self::Alphanumeric => value.chars().all(|ch| ch.is_ascii_alphanumeric()),
            Self::Uuid => is_valid_uuid(value),
            Self::Slug => is_valid_slug(value),
        }
    }

    /// Returns the canonical shorthand name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Num => "num",
            Self::Uuid => "uuid",
            Self::Slug => "slug",
            Self::Alpha => "alpha",
            Self::Alphanumeric => "alphanumeric",
        }
    }

    /// Returns the precedence rank among built-in constraints:
    /// Uuid (50) > Num (40) > Alpha (30) > Alphanumeric (20) > Slug (10).
    #[must_use]
    pub fn specificity_rank(&self) -> u8 {
        match self {
            Self::Uuid => 50,
            Self::Num => 40,
            Self::Alpha => 30,
            Self::Alphanumeric => 20,
            Self::Slug => 10,
        }
    }
}

/// Evaluates if a string matches UUID format (8-4-4-4-12 hex).
fn is_valid_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }

    let bytes = value.as_bytes();
    if bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-' || bytes[23] != b'-' {
        return false;
    }

    for (idx, &byte) in bytes.iter().enumerate() {
        if idx == 8 || idx == 13 || idx == 18 || idx == 23 {
            continue;
        }
        if !byte.is_ascii_hexdigit() {
            return false;
        }
    }

    true
}

/// Evaluates if a string matches URL slug format.
fn is_valid_slug(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.first() == Some(&b'-')
        || bytes.first() == Some(&b'_')
        || bytes.last() == Some(&b'-')
        || bytes.last() == Some(&b'_')
    {
        return false;
    }

    value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

/// Parameter validation constraint.
#[derive(Clone)]
pub enum Constraint {
    /// Fast built-in zero-allocation rule.
    Builtin(BuiltinConstraint),
    /// Regular expression pattern constraint.
    #[cfg(feature = "regex")]
    Regex {
        /// Compiled regex pattern.
        pattern: regex::Regex,
        /// Original pattern string for display and comparisons.
        source: String,
    },
    /// Custom predicate closure.
    Custom {
        /// Description of the rule.
        name: String,
        /// Predicate function.
        predicate: Arc<dyn Fn(&str) -> bool + Send + Sync + 'static>,
    },
}

impl Constraint {
    /// Creates a numeric constraint.
    #[must_use]
    pub fn num() -> Self {
        Self::Builtin(BuiltinConstraint::Num)
    }

    /// Creates a UUID constraint.
    #[must_use]
    pub fn uuid() -> Self {
        Self::Builtin(BuiltinConstraint::Uuid)
    }

    /// Creates a slug constraint.
    #[must_use]
    pub fn slug() -> Self {
        Self::Builtin(BuiltinConstraint::Slug)
    }

    /// Creates an alphabetic constraint.
    #[must_use]
    pub fn alpha() -> Self {
        Self::Builtin(BuiltinConstraint::Alpha)
    }

    /// Creates an alphanumeric constraint.
    #[must_use]
    pub fn alphanumeric() -> Self {
        Self::Builtin(BuiltinConstraint::Alphanumeric)
    }

    /// Creates a regex constraint from a pattern string.
    #[cfg(feature = "regex")]
    pub fn regex(pattern_str: &str) -> Result<Self, MatcherError> {
        let anchored = format!(r"\A(?:{pattern_str})\z");
        let compiled =
            regex::Regex::new(&anchored).map_err(|err| MatcherError::InvalidPattern {
                pattern: pattern_str.to_string(),
                reason: err.to_string(),
            })?;

        Ok(Self::Regex {
            pattern: compiled,
            source: pattern_str.to_string(),
        })
    }

    /// Creates a custom predicate constraint with a descriptive name.
    pub fn custom<F>(name: impl Into<String>, predicate: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        Self::Custom {
            name: name.into(),
            predicate: Arc::new(predicate),
        }
    }

    /// Evaluates if the given value satisfies this constraint.
    #[must_use]
    pub fn matches(&self, value: &str) -> bool {
        match self {
            Self::Builtin(builtin) => builtin.matches(value),
            #[cfg(feature = "regex")]
            Self::Regex { pattern, .. } => pattern.is_match(value),
            Self::Custom { predicate, .. } => predicate(value),
        }
    }

    /// Returns a human-readable description of the constraint.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::Builtin(builtin) => builtin.name().to_string(),
            #[cfg(feature = "regex")]
            Self::Regex { source, .. } => format!("regex({source})"),
            Self::Custom { name, .. } => name.clone(),
        }
    }

    /// Numeric precedence ranking: higher number means higher precedence.
    /// Regex (100) > Builtin (10..50) > Custom (1).
    #[must_use]
    pub fn precedence_rank(&self) -> u8 {
        match self {
            #[cfg(feature = "regex")]
            Self::Regex { .. } => 100,
            Self::Builtin(builtin) => builtin.specificity_rank(),
            Self::Custom { .. } => 1,
        }
    }
}

impl PartialEq for Constraint {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Builtin(a), Self::Builtin(b)) => a == b,
            #[cfg(feature = "regex")]
            (Self::Regex { source: a, .. }, Self::Regex { source: b, .. }) => a == b,
            (Self::Custom { name: a, .. }, Self::Custom { name: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl Eq for Constraint {}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Constraint")
            .field(&self.description())
            .finish()
    }
}
