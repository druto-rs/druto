//! Enum schema builder mapping string inputs directly into typed Rust enums.

use std::{fmt::Display, str::FromStr};

use serde_json::Value;

use crate::{
    context::ValidationContext,
    error::{RuleParams, ValidationError},
    messages::MessagesProvider,
    rule::RuleContext,
    schema::Schema,
};

/// Fluent schema builder for validating and parsing Rust enum types implementing `FromStr`.
#[derive(Clone)]
pub struct EnumSchema<T> {
    expected: Vec<String>,
    bail_on_fail: bool,
    inline_message: Option<String>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> EnumSchema<T>
where
    T: FromStr + Send + Sync + 'static,
{
    /// Creates a new `EnumSchema`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            expected: Vec::new(),
            bail_on_fail: false,
            inline_message: None,
            phantom: std::marker::PhantomData,
        }
    }

    /// Sets the list of expected variant string names for clear error reporting.
    #[must_use]
    pub fn variants(mut self, variants: &[&str]) -> Self {
        self.expected = variants.iter().map(|s| (*s).to_string()).collect();
        self
    }

    /// Configures short-circuiting on failure.
    #[must_use]
    pub fn bail(mut self, enabled: bool) -> Self {
        self.bail_on_fail = enabled;
        self
    }

    /// Sets an inline custom error message override.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inline_message = Some(message.into());
        self
    }
}

impl<T> Default for EnumSchema<T>
where
    T: FromStr + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Schema<T> for EnumSchema<T>
where
    T: FromStr + Send + Sync + 'static,
    T::Err: Display,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<T, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);
        let inline = self.inline_message.as_deref();

        let str_val = match value {
            Value::String(s) => s.as_str(),
            Value::Null => {
                let issue = rule_ctx.create_issue("required", RuleParams::new(), inline);
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
            _ => {
                let issue = rule_ctx.create_issue("required", RuleParams::new(), inline);
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
        };

        match T::from_str(str_val) {
            Ok(parsed_enum) => Ok(parsed_enum),
            Err(parse_err) => {
                let mut params = RuleParams::new();
                if !self.expected.is_empty() {
                    params.insert("expected", self.expected.join(", "));
                } else {
                    params.insert("expected", "valid enum variant");
                }
                params.insert("actual", str_val);
                params.insert("error", parse_err.to_string());

                let issue = rule_ctx.create_issue("enum", params, inline);
                Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ))
            }
        }
    }
}
