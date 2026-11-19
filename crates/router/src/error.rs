//! Strongly typed error taxonomy for routing, dispatch, parameter extraction, and execution.

use std::fmt;

use matcher::{MatcherError, Method};
use pipeline::PipelineError;
use qs::QsError;

/// Errors produced during route registration, path compilation, request dispatch, or handler execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
    /// No matching route was found for the requested URI path.
    NotFound,
    /// A route matched the path, but does not allow the requested HTTP method.
    MethodNotAllowed {
        /// The methods allowed for this path.
        allowed: Vec<Method>,
    },
    /// The incoming request body exceeded the maximum configured byte ceiling.
    PayloadTooLarge {
        /// The maximum allowed payload size in bytes.
        max_bytes: usize,
        /// The actual number of payload bytes received.
        received_bytes: usize,
    },
    /// A cryptographic URL signature was missing, expired, or invalid.
    InvalidSignature,
    /// Route parameter validation failed.
    InvalidParams {
        /// Detailed explanation of why parameters failed validation.
        message: String,
    },
    /// Request payload deserialization failed (e.g. malformed JSON or query parameters).
    DeserializationError(String),
    /// A route collides with an existing route definition during compilation.
    RouteConflict(String),
    /// An internal execution failure occurred.
    Internal(String),
    /// An error occurred during middleware pipeline dispatch.
    Pipeline(String),
}

impl fmt::Display for RouterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("404 Not Found"),
            Self::MethodNotAllowed { allowed } => {
                let methods = allowed
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(formatter, "405 Method Not Allowed (Allowed: {methods})")
            }
            Self::PayloadTooLarge {
                max_bytes,
                received_bytes,
            } => {
                write!(
                    formatter,
                    "413 Payload Too Large (Received {received_bytes} bytes, limit is {max_bytes} bytes)"
                )
            }
            Self::InvalidSignature => {
                formatter.write_str("403 Forbidden: Invalid or expired URL signature")
            }
            Self::InvalidParams { message } => write!(formatter, "400 Bad Request: {message}"),
            Self::DeserializationError(message) => {
                write!(formatter, "422 Unprocessable Entity: {message}")
            }
            Self::RouteConflict(message) => write!(formatter, "Route collision: {message}"),
            Self::Internal(message) => write!(formatter, "500 Internal Server Error: {message}"),
            Self::Pipeline(message) => write!(formatter, "Pipeline execution error: {message}"),
        }
    }
}

impl std::error::Error for RouterError {}

impl From<MatcherError> for RouterError {
    fn from(error: MatcherError) -> Self {
        match error {
            MatcherError::NotFound { .. } | MatcherError::RouteNotFound { .. } => Self::NotFound,
            MatcherError::MethodNotAllowed { allowed, .. } => Self::MethodNotAllowed { allowed },
            MatcherError::ConstraintFailed {
                param, constraint, ..
            } => Self::InvalidParams {
                message: format!("Parameter '{param}' failed constraint '{constraint}'"),
            },
            MatcherError::MissingParameter { name, route } => Self::InvalidParams {
                message: format!("Missing required parameter '{name}' for route '{route}'"),
            },
            MatcherError::ConflictingRoute {
                existing,
                new_pattern,
                reason,
            } => Self::RouteConflict(format!(
                "Pattern '{new_pattern}' conflicts with existing '{existing}': {reason}"
            )),
            MatcherError::InvalidPattern { pattern, reason } => {
                Self::RouteConflict(format!("Invalid pattern '{pattern}': {reason}"))
            }
            MatcherError::DecodeError { param, value } => Self::InvalidParams {
                message: format!(
                    "Failed to percent-decode parameter '{param}' with value '{value}'"
                ),
            },
        }
    }
}

impl From<PipelineError<Self>> for RouterError {
    fn from(error: PipelineError<Self>) -> Self {
        match error {
            PipelineError::NextAlreadyCalled => {
                Self::Pipeline("Middleware step next was invoked more than once".to_string())
            }
            PipelineError::Execution(inner) => inner,
        }
    }
}

impl From<serde_json::Error> for RouterError {
    fn from(error: serde_json::Error) -> Self {
        Self::DeserializationError(error.to_string())
    }
}

impl From<crypto::CryptoError> for RouterError {
    fn from(error: crypto::CryptoError) -> Self {
        match error {
            crypto::CryptoError::InvalidSignature
            | crypto::CryptoError::ExpiredSignature { .. } => Self::InvalidSignature,
            other => Self::Internal(other.to_string()),
        }
    }
}

impl From<cookie::CookieError> for RouterError {
    fn from(error: cookie::CookieError) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<QsError> for RouterError {
    fn from(error: QsError) -> Self {
        Self::DeserializationError(error.to_string())
    }
}

impl From<session::SessionError> for RouterError {
    fn from(error: session::SessionError) -> Self {
        Self::Internal(error.to_string())
    }
}
