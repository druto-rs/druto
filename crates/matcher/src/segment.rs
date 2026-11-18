//! AST representation and specificity ordering for route path segments.

use std::fmt;

use crate::constraint::Constraint;

/// Represents a single compiled path segment between slashes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Segment {
    /// Exact literal match, e.g. `users` or `posts`.
    Static(String),
    /// Named dynamic parameter, e.g. `:id`, `{id}`, `:id?`, or `:id(num)`.
    Param {
        /// Parameter name used for key lookup.
        name: String,
        /// Indicates if the segment is optional (`:id?`).
        optional: bool,
        /// Optional constraint limiting acceptable values.
        constraint: Option<Constraint>,
    },
    /// Catch-all wildcard matching the remainder of the URI path, e.g. `*path` or `**`.
    Wildcard {
        /// Parameter name capturing the wildcard path slice.
        name: String,
    },
}

impl Segment {
    /// Creates a static segment.
    #[must_use]
    pub fn static_segment(literal: impl Into<String>) -> Self {
        Self::Static(literal.into())
    }

    /// Creates an unconstrained required parameter segment.
    #[must_use]
    pub fn param(name: impl Into<String>) -> Self {
        Self::Param {
            name: name.into(),
            optional: false,
            constraint: None,
        }
    }

    /// Creates an unconstrained optional parameter segment.
    #[must_use]
    pub fn optional_param(name: impl Into<String>) -> Self {
        Self::Param {
            name: name.into(),
            optional: true,
            constraint: None,
        }
    }

    /// Creates a constrained parameter segment.
    #[must_use]
    pub fn constrained_param(
        name: impl Into<String>,
        optional: bool,
        constraint: Constraint,
    ) -> Self {
        Self::Param {
            name: name.into(),
            optional,
            constraint: Some(constraint),
        }
    }

    /// Creates a catch-all wildcard segment.
    #[must_use]
    pub fn wildcard(name: impl Into<String>) -> Self {
        Self::Wildcard { name: name.into() }
    }

    /// Returns the specificity rank for precedence sorting:
    /// Static (4) > Constrained Param (3) > Unconstrained Param (2) > Wildcard (1).
    #[must_use]
    pub fn specificity_rank(&self) -> u8 {
        match self {
            Self::Static(_) => 4,
            Self::Param {
                constraint: Some(_),
                ..
            } => 3,
            Self::Param {
                constraint: None, ..
            } => 2,
            Self::Wildcard { .. } => 1,
        }
    }

    /// Checks if two segments have identical matching specificity (potential collision).
    #[must_use]
    pub fn has_identical_specificity(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(a), Self::Static(b)) => a == b,
            (
                Self::Param {
                    constraint: None,
                    optional: opt_a,
                    ..
                },
                Self::Param {
                    constraint: None,
                    optional: opt_b,
                    ..
                },
            ) => opt_a == opt_b,
            (
                Self::Param {
                    constraint: Some(c_a),
                    optional: opt_a,
                    ..
                },
                Self::Param {
                    constraint: Some(c_b),
                    optional: opt_b,
                    ..
                },
            ) => opt_a == opt_b && c_a == c_b,
            (Self::Wildcard { .. }, Self::Wildcard { .. }) => true,
            _ => false,
        }
    }

    /// Returns true if this segment is an optional parameter.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        match self {
            Self::Param { optional, .. } => *optional,
            _ => false,
        }
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(val) => f.write_str(val),
            Self::Param {
                name,
                optional,
                constraint,
            } => {
                let opt_mark = if *optional { "?" } else { "" };
                if let Some(rule) = constraint {
                    write!(f, ":{name}({}){opt_mark}", rule.description())
                } else {
                    write!(f, ":{name}{opt_mark}")
                }
            }
            Self::Wildcard { name } => write!(f, "*{name}"),
        }
    }
}
