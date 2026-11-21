//! Number schemas for floating-point and integer validation with string coercion.

use std::sync::Arc;

use serde_json::Value;
#[cfg(feature = "async")]
use {crate::rule::AsyncCustomRule, futures_util::future::BoxFuture};

use crate::{
    coercion::coerce_to_number,
    context::ValidationContext,
    error::{RuleParams, ValidationError, ValidationIssue},
    messages::MessagesProvider,
    rule::{CustomRule, RuleContext},
    schema::Schema,
};

#[derive(Clone)]
enum NumberPredicate {
    Min(f64),
    Max(f64),
    Range(f64, f64),
    Integer,
    Positive,
    Negative,
    MultipleOf(f64),
    Custom(Arc<dyn CustomRule<f64>>),
}

/// Fluent schema builder for validating numbers (`f64`).
#[derive(Clone)]
pub struct NumberSchema {
    predicates: Vec<NumberPredicate>,
    bail_on_fail: bool,
    inline_message: Option<String>,
    #[cfg(feature = "async")]
    async_rules: Vec<Arc<dyn AsyncCustomRule<f64>>>,
}

impl Default for NumberSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl NumberSchema {
    /// Creates a new `NumberSchema`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            predicates: Vec::new(),
            bail_on_fail: false,
            inline_message: None,
            #[cfg(feature = "async")]
            async_rules: Vec::new(),
        }
    }

    /// Configures whether rule evaluation on this field should stop on the first failure.
    #[must_use]
    pub fn bail(mut self, enabled: bool) -> Self {
        self.bail_on_fail = enabled;
        self
    }

    /// Sets an inline custom error message override for this number field.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inline_message = Some(message.into());
        self
    }

    /// Enforces a minimum value constraint.
    #[must_use]
    pub fn min(mut self, minimum: f64) -> Self {
        self.predicates.push(NumberPredicate::Min(minimum));
        self
    }

    /// Enforces a maximum value constraint.
    #[must_use]
    pub fn max(mut self, maximum: f64) -> Self {
        self.predicates.push(NumberPredicate::Max(maximum));
        self
    }

    /// Enforces that the value falls within an inclusive numeric range `[min, max]`.
    #[must_use]
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.predicates.push(NumberPredicate::Range(min, max));
        self
    }

    /// Enforces that the number has no fractional component.
    #[must_use]
    pub fn int(mut self) -> Self {
        self.predicates.push(NumberPredicate::Integer);
        self
    }

    /// Enforces that the number is strictly positive (`> 0.0`).
    #[must_use]
    pub fn positive(mut self) -> Self {
        self.predicates.push(NumberPredicate::Positive);
        self
    }

    /// Enforces that the number is strictly negative (`< 0.0`).
    #[must_use]
    pub fn negative(mut self) -> Self {
        self.predicates.push(NumberPredicate::Negative);
        self
    }

    /// Enforces that the number is an exact multiple of the specified step.
    #[must_use]
    pub fn multiple_of(mut self, step: f64) -> Self {
        self.predicates.push(NumberPredicate::MultipleOf(step));
        self
    }

    /// Attaches a custom synchronous validation rule.
    #[must_use]
    pub fn rule<R: CustomRule<f64> + 'static>(mut self, rule: R) -> Self {
        self.predicates
            .push(NumberPredicate::Custom(Arc::new(rule)));
        self
    }

    #[cfg(feature = "async")]
    /// Attaches an asynchronous custom validation rule.
    #[must_use]
    pub fn rule_async<R: AsyncCustomRule<f64> + 'static>(mut self, rule: R) -> Self {
        self.async_rules.push(Arc::new(rule));
        self
    }

    fn evaluate_predicate(
        &self,
        predicate: &NumberPredicate,
        value: f64,
        rule_context: &RuleContext<'_>,
    ) -> Option<ValidationIssue> {
        let inline = self.inline_message.as_deref();

        match predicate {
            NumberPredicate::Min(min) => {
                if value < *min {
                    let mut params = RuleParams::new();
                    params.insert("min", min.to_string());
                    params.insert("actual", value.to_string());
                    Some(rule_context.create_issue("min", params, inline))
                } else {
                    None
                }
            }
            NumberPredicate::Max(max) => {
                if value > *max {
                    let mut params = RuleParams::new();
                    params.insert("max", max.to_string());
                    params.insert("actual", value.to_string());
                    Some(rule_context.create_issue("max", params, inline))
                } else {
                    None
                }
            }
            NumberPredicate::Range(min, max) => {
                if value < *min || value > *max {
                    let mut params = RuleParams::new();
                    params.insert("min", min.to_string());
                    params.insert("max", max.to_string());
                    params.insert("actual", value.to_string());
                    Some(rule_context.create_issue("range", params, inline))
                } else {
                    None
                }
            }
            NumberPredicate::Integer => {
                if value.fract() != 0.0 {
                    Some(rule_context.create_issue("integer", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            NumberPredicate::Positive => {
                if value <= 0.0 {
                    Some(rule_context.create_issue("positive", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            NumberPredicate::Negative => {
                if value >= 0.0 {
                    Some(rule_context.create_issue("negative", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            NumberPredicate::MultipleOf(step) => {
                let remainder = (value / *step).fract();
                if remainder.abs() > f64::EPSILON && (1.0 - remainder).abs() > f64::EPSILON {
                    let mut params = RuleParams::new();
                    params.insert("step", step.to_string());
                    Some(rule_context.create_issue("multiple_of", params, inline))
                } else {
                    None
                }
            }
            NumberPredicate::Custom(rule) => rule.validate(&value, rule_context).err(),
        }
    }
}

impl Schema<f64> for NumberSchema {
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<f64, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);

        let parsed_num = match coerce_to_number(value) {
            Some(num) => num.as_f64().unwrap_or(0.0),
            None => {
                let rule_name = if value.is_null() {
                    "required"
                } else {
                    "number"
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

        let mut errors = ValidationError::new();
        for predicate in &self.predicates {
            if let Some(issue) = self.evaluate_predicate(predicate, parsed_num, &rule_ctx) {
                errors.push(issue);
                if self.bail_on_fail {
                    break;
                }
            }
        }

        if errors.is_empty() {
            Ok(parsed_num)
        } else {
            Err(errors)
        }
    }

    #[cfg(feature = "async")]
    fn validate_async_path<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<f64, ValidationError>> {
        Box::pin(async move {
            let num = self.validate_path(value, path, context, parent_data, messages)?;
            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let mut errors = ValidationError::new();

            for async_rule in &self.async_rules {
                match async_rule.validate(&num, &rule_ctx).await {
                    Ok(()) => {}
                    Err(issue) => {
                        errors.push(issue);
                        if self.bail_on_fail {
                            break;
                        }
                    }
                }
            }

            if errors.is_empty() {
                Ok(num)
            } else {
                Err(errors)
            }
        })
    }
}

/// Dedicated integer schema builder validating into `i64`.
#[derive(Clone)]
pub struct IntegerSchema {
    inner: NumberSchema,
}

impl Default for IntegerSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegerSchema {
    /// Creates a new `IntegerSchema`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: NumberSchema::new().int(),
        }
    }

    /// Enforces a minimum integer constraint.
    #[must_use]
    pub fn min(mut self, minimum: i64) -> Self {
        self.inner = self.inner.min(minimum as f64);
        self
    }

    /// Enforces a maximum integer constraint.
    #[must_use]
    pub fn max(mut self, maximum: i64) -> Self {
        self.inner = self.inner.max(maximum as f64);
        self
    }

    /// Enforces an inclusive integer range `[min, max]`.
    #[must_use]
    pub fn range(mut self, min: i64, max: i64) -> Self {
        self.inner = self.inner.range(min as f64, max as f64);
        self
    }

    /// Enforces that the integer is strictly positive (`> 0`).
    #[must_use]
    pub fn positive(mut self) -> Self {
        self.inner = self.inner.positive();
        self
    }

    /// Enforces that the integer is strictly negative (`< 0`).
    #[must_use]
    pub fn negative(mut self) -> Self {
        self.inner = self.inner.negative();
        self
    }

    /// Enforces that the integer is a multiple of `step`.
    #[must_use]
    pub fn multiple_of(mut self, step: i64) -> Self {
        self.inner = self.inner.multiple_of(step as f64);
        self
    }

    /// Configures short-circuiting on first failure.
    #[must_use]
    pub fn bail(mut self, enabled: bool) -> Self {
        self.inner = self.inner.bail(enabled);
        self
    }

    /// Sets an inline custom message override.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inner = self.inner.with_message(message);
        self
    }
}

impl Schema<i64> for IntegerSchema {
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<i64, ValidationError> {
        let float_val = self
            .inner
            .validate_path(value, path, context, parent_data, messages)?;
        Ok(float_val as i64)
    }

    #[cfg(feature = "async")]
    fn validate_async_path<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<i64, ValidationError>> {
        Box::pin(async move {
            let float_val = self
                .inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await?;
            Ok(float_val as i64)
        })
    }
}
