//! Tuple schema builders for fixed-length heterogeneous sequences.

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

/// Tuple schema validating a 2-element heterogeneous sequence `(T1, T2)`.
#[derive(Clone)]
pub struct Tuple2Schema<S1, S2, T1, T2> {
    first: S1,
    second: S2,
    bail_on_fail: bool,
    inline_message: Option<String>,
    phantom: std::marker::PhantomData<(T1, T2)>,
}

impl<S1, S2, T1, T2> Tuple2Schema<S1, S2, T1, T2>
where
    S1: Schema<T1>,
    S2: Schema<T2>,
{
    /// Creates a new `Tuple2Schema`.
    #[must_use]
    pub fn new(first: S1, second: S2) -> Self {
        Self {
            first,
            second,
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
}

impl<S1, S2, T1, T2> Schema<(T1, T2)> for Tuple2Schema<S1, S2, T1, T2>
where
    S1: Schema<T1>,
    S2: Schema<T2>,
    T1: Send + Sync + 'static,
    T2: Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<(T1, T2), ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);
        let inline = self.inline_message.as_deref();

        let items = match value {
            Value::Array(arr) if arr.len() == 2 => arr,
            _ => {
                let mut params = RuleParams::new();
                params.insert("length", "2");
                let issue = rule_ctx.create_issue("exact_items", params, inline);
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
        };

        let mut errors = ValidationError::new();

        let path1 = if path.is_empty() {
            "[0]".to_string()
        } else {
            format!("{path}[0]")
        };
        let path2 = if path.is_empty() {
            "[1]".to_string()
        } else {
            format!("{path}[1]")
        };

        let res1 = self
            .first
            .validate_path(&items[0], &path1, context, Some(value), messages);
        let res2 = self
            .second
            .validate_path(&items[1], &path2, context, Some(value), messages);

        let valid1 = match res1 {
            Ok(val) => Some(val),
            Err(err) => {
                errors.merge(err);
                if self.bail_on_fail {
                    return Err(errors);
                }
                None
            }
        };

        let valid2 = match res2 {
            Ok(val) => Some(val),
            Err(err) => {
                errors.merge(err);
                if self.bail_on_fail {
                    return Err(errors);
                }
                None
            }
        };

        if let (Some(v1), Some(v2)) = (valid1, valid2) {
            Ok((v1, v2))
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
    ) -> BoxFuture<'a, Result<(T1, T2), ValidationError>> {
        Box::pin(async move {
            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let inline = self.inline_message.as_deref();

            let items = match value {
                Value::Array(arr) if arr.len() == 2 => arr,
                _ => {
                    let mut params = RuleParams::new();
                    params.insert("length", "2");
                    let issue = rule_ctx.create_issue("exact_items", params, inline);
                    return Err(ValidationError::single(
                        issue.field,
                        issue.rule,
                        issue.message,
                    ));
                }
            };

            let mut errors = ValidationError::new();

            let path1 = if path.is_empty() {
                "[0]".to_string()
            } else {
                format!("{path}[0]")
            };
            let path2 = if path.is_empty() {
                "[1]".to_string()
            } else {
                format!("{path}[1]")
            };

            let res1 = self
                .first
                .validate_async_path(&items[0], &path1, context, Some(value), messages)
                .await;
            let res2 = self
                .second
                .validate_async_path(&items[1], &path2, context, Some(value), messages)
                .await;

            let valid1 = match res1 {
                Ok(val) => Some(val),
                Err(err) => {
                    errors.merge(err);
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                    None
                }
            };

            let valid2 = match res2 {
                Ok(val) => Some(val),
                Err(err) => {
                    errors.merge(err);
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                    None
                }
            };

            if let (Some(v1), Some(v2)) = (valid1, valid2) {
                Ok((v1, v2))
            } else {
                Err(errors)
            }
        })
    }
}
