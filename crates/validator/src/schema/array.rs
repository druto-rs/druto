//! Array schema builder for validating collections and sequential lists.

use std::collections::HashSet;

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

/// Fluent schema builder for validating collections of items (`Vec<T>`).
#[derive(Clone)]
pub struct ArraySchema<S, T> {
    element_schema: S,
    min_items: Option<usize>,
    max_items: Option<usize>,
    exact_items: Option<usize>,
    not_empty: bool,
    distinct: bool,
    bail_on_fail: bool,
    inline_message: Option<String>,
    phantom: std::marker::PhantomData<T>,
}

impl<S, T> ArraySchema<S, T>
where
    S: Schema<T>,
{
    /// Creates a new `ArraySchema` wrapping an element schema.
    #[must_use]
    pub fn new(element_schema: S) -> Self {
        Self {
            element_schema,
            min_items: None,
            max_items: None,
            exact_items: None,
            not_empty: false,
            distinct: false,
            bail_on_fail: false,
            inline_message: None,
            phantom: std::marker::PhantomData,
        }
    }

    /// Enforces a minimum number of elements.
    #[must_use]
    pub fn min_items(mut self, min: usize) -> Self {
        self.min_items = Some(min);
        self
    }

    /// Enforces a maximum number of elements.
    #[must_use]
    pub fn max_items(mut self, max: usize) -> Self {
        self.max_items = Some(max);
        self
    }

    /// Enforces an exact number of elements.
    #[must_use]
    pub fn exact_items(mut self, length: usize) -> Self {
        self.exact_items = Some(length);
        self
    }

    /// Enforces that the array contains at least one item.
    #[must_use]
    pub fn not_empty(mut self) -> Self {
        self.not_empty = true;
        self
    }

    /// Enforces that all items in the array are distinct (no duplicates).
    #[must_use]
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
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

impl<S, T> Schema<Vec<T>> for ArraySchema<S, T>
where
    S: Schema<T>,
    T: Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<Vec<T>, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);
        let inline = self.inline_message.as_deref();

        let items = match value {
            Value::Array(arr) => arr,
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

        if self.not_empty && items.is_empty() {
            errors.push(rule_ctx.create_issue("not_empty", RuleParams::new(), inline));
            if self.bail_on_fail {
                return Err(errors);
            }
        }

        if let Some(min) = self.min_items
            && items.len() < min
        {
            let mut params = RuleParams::new();
            params.insert("min", min.to_string());
            params.insert("actual", items.len().to_string());
            errors.push(rule_ctx.create_issue("min_items", params, inline));
            if self.bail_on_fail {
                return Err(errors);
            }
        }

        if let Some(max) = self.max_items
            && items.len() > max
        {
            let mut params = RuleParams::new();
            params.insert("max", max.to_string());
            params.insert("actual", items.len().to_string());
            errors.push(rule_ctx.create_issue("max_items", params, inline));
            if self.bail_on_fail {
                return Err(errors);
            }
        }

        if let Some(exact) = self.exact_items
            && items.len() != exact
        {
            let mut params = RuleParams::new();
            params.insert("length", exact.to_string());
            params.insert("actual", items.len().to_string());
            errors.push(rule_ctx.create_issue("exact_items", params, inline));
            if self.bail_on_fail {
                return Err(errors);
            }
        }

        if self.distinct {
            let mut seen = HashSet::new();
            for item in items {
                let serialized = item.to_string();
                if !seen.insert(serialized) {
                    errors.push(rule_ctx.create_issue("distinct", RuleParams::new(), inline));
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                    break;
                }
            }
        }

        let mut validated_items = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            let item_path = if path.is_empty() {
                format!("[{index}]")
            } else {
                format!("{path}[{index}]")
            };

            match self.element_schema.validate_path(
                item,
                &item_path,
                context,
                Some(value),
                messages,
            ) {
                Ok(valid_elem) => validated_items.push(valid_elem),
                Err(element_errors) => {
                    errors.merge(element_errors);
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(validated_items)
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
    ) -> BoxFuture<'a, Result<Vec<T>, ValidationError>> {
        Box::pin(async move {
            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let inline = self.inline_message.as_deref();

            let items = match value {
                Value::Array(arr) => arr,
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

            if self.not_empty && items.is_empty() {
                errors.push(rule_ctx.create_issue("not_empty", RuleParams::new(), inline));
                if self.bail_on_fail {
                    return Err(errors);
                }
            }

            if let Some(min) = self.min_items
                && items.len() < min
            {
                let mut params = RuleParams::new();
                params.insert("min", min.to_string());
                params.insert("actual", items.len().to_string());
                errors.push(rule_ctx.create_issue("min_items", params, inline));
                if self.bail_on_fail {
                    return Err(errors);
                }
            }

            if let Some(max) = self.max_items
                && items.len() > max
            {
                let mut params = RuleParams::new();
                params.insert("max", max.to_string());
                params.insert("actual", items.len().to_string());
                errors.push(rule_ctx.create_issue("max_items", params, inline));
                if self.bail_on_fail {
                    return Err(errors);
                }
            }

            if let Some(exact) = self.exact_items
                && items.len() != exact
            {
                let mut params = RuleParams::new();
                params.insert("length", exact.to_string());
                params.insert("actual", items.len().to_string());
                errors.push(rule_ctx.create_issue("exact_items", params, inline));
                if self.bail_on_fail {
                    return Err(errors);
                }
            }

            if self.distinct {
                let mut seen = HashSet::new();
                for item in items {
                    let serialized = item.to_string();
                    if !seen.insert(serialized) {
                        errors.push(rule_ctx.create_issue("distinct", RuleParams::new(), inline));
                        if self.bail_on_fail {
                            return Err(errors);
                        }
                        break;
                    }
                }
            }

            let mut validated_items = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                let item_path = if path.is_empty() {
                    format!("[{index}]")
                } else {
                    format!("{path}[{index}]")
                };

                match self
                    .element_schema
                    .validate_async_path(item, &item_path, context, Some(value), messages)
                    .await
                {
                    Ok(valid_elem) => validated_items.push(valid_elem),
                    Err(element_errors) => {
                        errors.merge(element_errors);
                        if self.bail_on_fail {
                            return Err(errors);
                        }
                    }
                }
            }

            if errors.is_empty() {
                Ok(validated_items)
            } else {
                Err(errors)
            }
        })
    }
}
