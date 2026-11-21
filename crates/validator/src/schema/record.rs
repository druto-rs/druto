//! Record schema builder for validating dynamic key-value maps.

use std::collections::BTreeMap;

#[cfg(feature = "async")]
use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::{
    context::ValidationContext,
    error::{RuleParams, ValidationError},
    messages::MessagesProvider,
    rule::RuleContext,
    schema::Schema,
};

/// Fluent schema builder for validating key-value dictionaries (`BTreeMap<String, V>`).
#[derive(Clone)]
pub struct RecordSchema<S, V> {
    value_schema: S,
    bail_on_fail: bool,
    inline_message: Option<String>,
    phantom: std::marker::PhantomData<V>,
}

impl<S, V> RecordSchema<S, V>
where
    S: Schema<V>,
{
    /// Creates a new `RecordSchema` with the specified value schema.
    #[must_use]
    pub fn new(value_schema: S) -> Self {
        Self {
            value_schema,
            bail_on_fail: false,
            inline_message: None,
            phantom: std::marker::PhantomData,
        }
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

impl<S, V> Schema<BTreeMap<String, V>> for RecordSchema<S, V>
where
    S: Schema<V>,
    V: Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<BTreeMap<String, V>, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);
        let inline = self.inline_message.as_deref();

        let map = match value {
            Value::Object(obj) => obj,
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

        let mut errors = ValidationError::new();
        let mut validated_map = BTreeMap::new();

        for (key, val) in map {
            let entry_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };

            match self
                .value_schema
                .validate_path(val, &entry_path, context, Some(value), messages)
            {
                Ok(valid_val) => {
                    validated_map.insert(key.clone(), valid_val);
                }
                Err(entry_err) => {
                    errors.merge(entry_err);
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(validated_map)
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
    ) -> BoxFuture<'a, Result<BTreeMap<String, V>, ValidationError>> {
        Box::pin(async move {
            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let inline = self.inline_message.as_deref();

            let map = match value {
                Value::Object(obj) => obj,
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

            let mut errors = ValidationError::new();
            let mut validated_map = BTreeMap::new();

            for (key, val) in map {
                let entry_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };

                match self
                    .value_schema
                    .validate_async_path(val, &entry_path, context, Some(value), messages)
                    .await
                {
                    Ok(valid_val) => {
                        validated_map.insert(key.clone(), valid_val);
                    }
                    Err(entry_err) => {
                        errors.merge(entry_err);
                        if self.bail_on_fail {
                            return Err(errors);
                        }
                    }
                }
            }

            if errors.is_empty() {
                Ok(validated_map)
            } else {
                Err(errors)
            }
        })
    }
}
