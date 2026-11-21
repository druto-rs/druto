//! Custom validation rule contracts, rule execution contexts, and async predicates.

use std::sync::Arc;

#[cfg(feature = "async")]
use futures_util::future::BoxFuture;

use crate::{
    context::ValidationContext,
    error::{RuleParams, ValidationIssue},
    messages::MessagesProvider,
};

/// Ambient execution context passed to custom validation predicates and refinement closures.
pub struct RuleContext<'a> {
    path: &'a str,
    context: &'a ValidationContext,
    parent_data: Option<&'a serde_json::Value>,
    messages: &'a dyn MessagesProvider,
}

impl<'a> RuleContext<'a> {
    /// Creates a new `RuleContext`.
    #[must_use]
    pub fn new(
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a serde_json::Value>,
        messages: &'a dyn MessagesProvider,
    ) -> Self {
        Self {
            path,
            context,
            parent_data,
            messages,
        }
    }

    /// Returns the canonical field path currently being validated.
    #[must_use]
    pub fn path(&self) -> &str {
        self.path
    }

    /// Returns the ambient typemap context.
    #[must_use]
    pub fn context(&self) -> &ValidationContext {
        self.context
    }

    /// Retrieves an ambient metadata entry from the context.
    #[must_use]
    pub fn get_ambient<T: 'static>(&self) -> Option<&T> {
        self.context.get::<T>()
    }

    /// Returns the parent object's JSON payload if currently validating an object field.
    #[must_use]
    pub fn parent_data(&self) -> Option<&serde_json::Value> {
        self.parent_data
    }

    /// Looks up a sibling field's value within the parent object.
    #[must_use]
    pub fn get_sibling(&self, field_name: &str) -> Option<&serde_json::Value> {
        self.parent_data
            .and_then(|parent| parent.as_object())
            .and_then(|map| map.get(field_name))
    }

    /// Formats a validation issue using the active message provider and cascading resolution.
    #[must_use]
    pub fn create_issue(
        &self,
        rule: &str,
        params: RuleParams,
        inline_override: Option<&str>,
    ) -> ValidationIssue {
        let message = self
            .messages
            .format_message(self.path, rule, &params, inline_override);
        ValidationIssue {
            field: self.path.to_string(),
            rule: rule.to_string(),
            message,
            params,
        }
    }
}

/// Synchronous custom validation rule contract.
pub trait CustomRule<T>: Send + Sync {
    /// Validates a typed value, returning `Ok(())` on success or an issue on failure.
    fn validate(&self, value: &T, ctx: &RuleContext<'_>) -> Result<(), ValidationIssue>;
}

impl<T, F> CustomRule<T> for F
where
    F: Fn(&T, &RuleContext<'_>) -> Result<(), ValidationIssue> + Send + Sync,
{
    fn validate(&self, value: &T, ctx: &RuleContext<'_>) -> Result<(), ValidationIssue> {
        self(value, ctx)
    }
}

/// Shared thread-safe reference to a custom rule.
pub type SharedCustomRule<T> = Arc<dyn CustomRule<T>>;

/// Asynchronous custom validation rule contract.
#[cfg(feature = "async")]
pub trait AsyncCustomRule<T>: Send + Sync {
    /// Validates a typed value asynchronously, returning a future resolving to `Result<(), ValidationIssue>`.
    fn validate<'a>(
        &'a self,
        value: &'a T,
        ctx: &'a RuleContext<'a>,
    ) -> BoxFuture<'a, Result<(), ValidationIssue>>;
}

#[cfg(feature = "async")]
impl<T, F> AsyncCustomRule<T> for F
where
    F: (for<'a> Fn(&'a T, &'a RuleContext<'a>) -> BoxFuture<'a, Result<(), ValidationIssue>>)
        + Send
        + Sync,
{
    fn validate<'a>(
        &'a self,
        value: &'a T,
        ctx: &'a RuleContext<'a>,
    ) -> BoxFuture<'a, Result<(), ValidationIssue>> {
        self(value, ctx)
    }
}

#[cfg(feature = "async")]
/// Shared thread-safe reference to an asynchronous custom rule.
pub type SharedAsyncCustomRule<T> = Arc<dyn AsyncCustomRule<T>>;
