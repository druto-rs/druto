//! Union schema combinators for validating multiple alternative schemas.

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

/// Binary union schema combinator returning `T` if either `S1` or `S2` passes.
#[derive(Clone)]
pub struct Union2Schema<S1, S2, T> {
    first: S1,
    second: S2,
    inline_message: Option<String>,
    phantom: std::marker::PhantomData<T>,
}

impl<S1, S2, T> Union2Schema<S1, S2, T>
where
    S1: Schema<T>,
    S2: Schema<T>,
{
    /// Creates a new `Union2Schema`.
    #[must_use]
    pub fn new(first: S1, second: S2) -> Self {
        Self {
            first,
            second,
            inline_message: None,
            phantom: std::marker::PhantomData,
        }
    }

    /// Sets an inline custom message override.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inline_message = Some(message.into());
        self
    }
}

impl<S1, S2, T> Schema<T> for Union2Schema<S1, S2, T>
where
    S1: Schema<T>,
    S2: Schema<T>,
    T: Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<T, ValidationError> {
        match self
            .first
            .validate_path(value, path, context, parent_data, messages)
        {
            Ok(val) => Ok(val),
            Err(first_err) => {
                match self
                    .second
                    .validate_path(value, path, context, parent_data, messages)
                {
                    Ok(val) => Ok(val),
                    Err(second_err) => {
                        let rule_ctx = RuleContext::new(path, context, parent_data, messages);
                        let mut combined = first_err;
                        combined.merge(second_err);
                        if let Some(msg) = &self.inline_message {
                            let issue =
                                rule_ctx.create_issue("union", RuleParams::new(), Some(msg));
                            Err(ValidationError::single(
                                issue.field,
                                issue.rule,
                                issue.message,
                            ))
                        } else {
                            Err(combined)
                        }
                    }
                }
            }
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
    ) -> BoxFuture<'a, Result<T, ValidationError>> {
        Box::pin(async move {
            match self
                .first
                .validate_async_path(value, path, context, parent_data, messages)
                .await
            {
                Ok(val) => Ok(val),
                Err(first_err) => {
                    match self
                        .second
                        .validate_async_path(value, path, context, parent_data, messages)
                        .await
                    {
                        Ok(val) => Ok(val),
                        Err(second_err) => {
                            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
                            let mut combined = first_err;
                            combined.merge(second_err);
                            if let Some(msg) = &self.inline_message {
                                let issue =
                                    rule_ctx.create_issue("union", RuleParams::new(), Some(msg));
                                Err(ValidationError::single(
                                    issue.field,
                                    issue.rule,
                                    issue.message,
                                ))
                            } else {
                                Err(combined)
                            }
                        }
                    }
                }
            }
        })
    }
}
