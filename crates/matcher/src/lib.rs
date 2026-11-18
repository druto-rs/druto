//! Route pattern compilation, trie indexing, parameter extraction, and HTTP method matching.
//!
//! # Architecture
//!
//! `matcher` provides high-performance URL pattern matching and parameter extraction for the Druto web framework.
//!
//! - **Dual Syntax**: Supports colon syntax (`:id`, `:id?`, `*wildcard`) and brace syntax (`{id}`, `{id?}`, `{wildcard*}`).
//! - **Deterministic Precedence**: Static > Regex Constraints > Built-in Shorthand Constraints > Dynamic Parameters > Wildcards.
//! - **Zero-Allocation Lookups**: Up to 16 path parameters are stored directly on the stack with borrowed `&'a str` slices.
//! - **Comprehensive HTTP Methods**: Supports standard RFC 9110 methods, RFC 10008 `QUERY`, CDN `PURGE`, WebDAV methods, and custom verbs with automatic 405 Method Not Allowed handling.
//! - **Reverse URL Generation**: Named route path synthesis (`build_path`) with constraint and parameter validation.

pub mod constraint;
pub mod error;
pub mod matcher;
pub mod method;
pub mod method_matcher;
pub mod params;
pub mod parser;
pub mod segment;
pub mod tree;

pub use constraint::{BuiltinConstraint, Constraint};
pub use error::MatcherError;
pub use matcher::{Match, Matcher, MatcherConfig, RouteBuilder};
pub use method::Method;
pub use method_matcher::{MethodMatch, MethodMatcher};
pub use params::{Params, QueryParams};
pub use parser::RouteParser;
pub use segment::Segment;
