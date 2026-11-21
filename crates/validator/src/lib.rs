//! Request schema definition, validation rules, sanitization, and structured error formatting for Druto.
//!
//! # Overview
//!
//! The `validator` crate provides universal schema definition, input sanitization, data coercion,
//! validation rule evaluation, and hierarchical field-level error reporting. It operates both as an
//! HTTP-agnostic data validation engine and as a multi-source HTTP request validator.
//!
//! # Architecture & Features
//!
//! - **Universal & Decoupled Core**: Validates arbitrary in-memory data structures, JSON values,
//!   or query string representations with zero I/O, compiling cleanly to `wasm32-unknown-unknown`.
//! - **"Parse, Don't Validate"**: Normalizes untrusted, weakly-typed input (trimming strings,
//!   empty-string-to-null conversion, query string coercion) and decodes directly into strictly typed Rust models.
//! - **Fail-Closed Security**: Silently strips undeclared fields by default (`strip`) to prevent
//!   mass-assignment vulnerabilities, with opt-in `.strict()` and `.passthrough()` modes.
//! - **Tri-Layer Authoring**:
//!   1. Fluent Schema Builders (`schema::object()`, `schema::string()`).
//!   2. Declarative Macro DSL ([`schema!`], [`request_schema!`]).
//!   3. Struct Derive Macro (`#[derive(Validate)]`).
//! - **Dual-Mode Execution**: Pure schemas execute synchronously with zero runtime overhead via
//!   [`Schema::validate`], while asynchronous rules are enabled under the `async` feature flag.
//! - **Cross-Field Relationships**: Native combinators (`.same_as()`, `.required_if()`, `.after_field()`, `.when()`).
//! - **Ambient Metadata (`ValidationContext`)**: Typemap container allowing custom rules to inspect
//!   runtime metadata (such as current user ID or tenant ID).
//! - **HTTP Content Negotiation**: Automatically formats `422 Unprocessable Entity` responses as
//!   either simple JSON error arrays (`Accept: application/json`) or RFC 9457 Problem Details (`Accept: application/problem+json`).

pub mod coercion;
pub mod context;
pub mod error;
pub mod messages;
pub mod request;
pub mod rule;
pub mod schema;

#[cfg(feature = "macros")]
pub mod macros;

#[cfg(feature = "router")]
pub mod router;

pub use context::ValidationContext;
pub use error::{RuleParams, ValidationError, ValidationIssue};
pub use messages::{MessagesProvider, SimpleMessagesProvider};
pub use request::{RequestParts, RequestSchema, ValidatedRequest, request};
#[cfg(feature = "router")]
pub use router::HttpContextValidateExt;
#[cfg(feature = "async")]
pub use rule::{AsyncCustomRule, SharedAsyncCustomRule};
pub use rule::{CustomRule, RuleContext, SharedCustomRule};
pub use schema::{
    Schema, UnknownFieldPolicy, array, boolean, r#enum, integer, number, object, record, string,
    tuple2, union2,
};

/// Convenient prelude module importing essential traits, builders, and types.
pub mod prelude {
    #[cfg(feature = "router")]
    pub use crate::HttpContextValidateExt;
    #[cfg(feature = "async")]
    pub use crate::{AsyncCustomRule, SharedAsyncCustomRule};
    pub use crate::{
        CustomRule, MessagesProvider, RequestParts, RequestSchema, RuleContext, RuleParams, Schema,
        SimpleMessagesProvider, UnknownFieldPolicy, ValidatedRequest, ValidationContext,
        ValidationError, ValidationIssue, array, boolean, r#enum, integer, number, object, record,
        request, string, tuple2, union2,
    };
    #[cfg(feature = "macros")]
    pub use crate::{request_schema, schema};
}
