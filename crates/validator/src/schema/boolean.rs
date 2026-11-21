//! Boolean schema builder with string coercion and acceptance validation.

use serde_json::Value;

use crate::{
    coercion::coerce_to_bool,
    context::ValidationContext,
    error::{RuleParams, ValidationError},
    messages::MessagesProvider,
    rule::RuleContext,
    schema::Schema,
};

/// Fluent schema builder for validating boolean values.
#[derive(Clone, Default)]
pub struct BooleanSchema {
    must_be_accepted: bool,
    bail_on_fail: bool,
    inline_message: Option<String>,
}

impl BooleanSchema {
    /// Creates a new `BooleanSchema`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            must_be_accepted: false,
            bail_on_fail: false,
            inline_message: None,
        }
    }

    /// Enforces that the boolean must be truthy (`true`, `"1"`, `"yes"`, `"on"`).
    ///
    /// Useful for terms of service agreements and mandatory consent checkboxes.
    #[must_use]
    pub fn accepted(mut self) -> Self {
        self.must_be_accepted = true;
        self
    }

    /// Configures short-circuiting on failure.
    #[must_use]
    pub fn bail(mut self, enabled: bool) -> Self {
        self.bail_on_fail = enabled;
        self
    }

    /// Sets an inline custom message override.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inline_message = Some(message.into());
        self
    }
}

impl Schema<bool> for BooleanSchema {
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<bool, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);

        let parsed_bool = match coerce_to_bool(value) {
            Some(b) => b,
            None => {
                let rule_name = if value.is_null() {
                    "required"
                } else {
                    "boolean"
                };
                let issue = rule_ctx.create_issue(
                    rule_name,
                    RuleParams::new(),
                    self.inline_message.as_deref(),
                );
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
        };

        if self.must_be_accepted && !parsed_bool {
            let issue = rule_ctx.create_issue(
                "accepted",
                RuleParams::new(),
                self.inline_message.as_deref(),
            );
            return Err(ValidationError::single(
                issue.field,
                issue.rule,
                issue.message,
            ));
        }

        Ok(parsed_bool)
    }
}
