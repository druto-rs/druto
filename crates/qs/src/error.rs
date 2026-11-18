//! Error types for query string parsing and serialization.

use std::fmt;

/// Errors that can occur during query string parsing, serialization, or deserialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QsError {
    /// Depth limit was exceeded during nested bracket parsing.
    DepthLimitExceeded(usize),
    /// Parameter count limit was exceeded.
    ParameterLimitExceeded(usize),
    /// Array index limit was exceeded.
    ArrayLimitExceeded(usize),
    /// Query string contains invalid syntax.
    InvalidSyntax(String),
    /// Custom error message, often from Serde deserialization or serialization.
    Custom(String),
}

impl fmt::Display for QsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DepthLimitExceeded(limit) => {
                write!(
                    formatter,
                    "depth limit exceeded: input nesting exceeds depth of {limit}"
                )
            }
            Self::ParameterLimitExceeded(limit) => {
                write!(
                    formatter,
                    "parameter limit exceeded: only {limit} parameters allowed"
                )
            }
            Self::ArrayLimitExceeded(limit) => {
                write!(
                    formatter,
                    "array limit exceeded: only {limit} elements allowed in array"
                )
            }
            Self::InvalidSyntax(message) => {
                write!(formatter, "invalid query string syntax: {message}")
            }
            Self::Custom(message) => {
                write!(formatter, "{message}")
            }
        }
    }
}

impl std::error::Error for QsError {}

impl serde::de::Error for QsError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Custom(message.to_string())
    }
}

impl serde::ser::Error for QsError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Custom(message.to_string())
    }
}
