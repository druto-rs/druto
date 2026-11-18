//! Error types for route pattern compilation and path matching.

use std::fmt;

use crate::method::Method;

/// Errors produced during route parsing, registration, matching, or path synthesis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatcherError {
    /// A route collides with an existing route with identical specificity.
    ConflictingRoute {
        /// The existing registered pattern.
        existing: String,
        /// The new conflicting pattern.
        new_pattern: String,
        /// Detail explaining the collision reason.
        reason: String,
    },
    /// The route pattern contains invalid syntax.
    InvalidPattern {
        /// The malformed pattern.
        pattern: String,
        /// Explanation of syntax error.
        reason: String,
    },
    /// A parameter failed constraint validation.
    ConstraintFailed {
        /// Name of the parameter that failed.
        param: String,
        /// The value that failed validation.
        value: String,
        /// The constraint rule description.
        constraint: String,
    },
    /// A required parameter is missing during reverse path generation.
    MissingParameter {
        /// Name of the missing parameter.
        name: String,
        /// The route name or pattern being built.
        route: String,
    },
    /// The requested named route was not found.
    RouteNotFound {
        /// Name of the route looked up.
        name: String,
    },
    /// No route matched the given path.
    NotFound {
        /// The requested path.
        path: String,
    },
    /// A route matched the path, but does not accept the requested HTTP method.
    MethodNotAllowed {
        /// The HTTP method requested.
        method: Method,
        /// The path requested.
        path: String,
        /// The methods allowed for this path.
        allowed: Vec<Method>,
    },
    /// Failed to percent-decode a parameter value.
    DecodeError {
        /// Parameter name.
        param: String,
        /// Malformed percent-encoded slice.
        value: String,
    },
}

impl fmt::Display for MatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingRoute {
                existing,
                new_pattern,
                reason,
            } => {
                write!(
                    f,
                    "route conflict: pattern `{new_pattern}` collides with existing `{existing}` ({reason})"
                )
            }
            Self::InvalidPattern { pattern, reason } => {
                write!(f, "invalid route pattern `{pattern}`: {reason}")
            }
            Self::ConstraintFailed {
                param,
                value,
                constraint,
            } => {
                write!(
                    f,
                    "constraint `{constraint}` failed for parameter `{param}` with value `{value}`"
                )
            }
            Self::MissingParameter { name, route } => {
                write!(
                    f,
                    "missing required parameter `{name}` when building path for route `{route}`"
                )
            }
            Self::RouteNotFound { name } => {
                write!(f, "named route `{name}` not found in matcher")
            }
            Self::NotFound { path } => {
                write!(f, "no route matched path `{path}`")
            }
            Self::MethodNotAllowed {
                method,
                path,
                allowed,
            } => {
                let allowed_str = allowed
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "method `{}` not allowed for path `{path}` (allowed: [{allowed_str}])",
                    method.as_str()
                )
            }
            Self::DecodeError { param, value } => {
                write!(
                    f,
                    "failed to percent-decode value `{value}` for parameter `{param}`"
                )
            }
        }
    }
}

impl std::error::Error for MatcherError {}
