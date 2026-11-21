//! Type transformation and refinement schema wrappers.

use std::sync::Arc;

#[cfg(feature = "async")]
use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::{
    context::ValidationContext,
    error::{RuleParams, ValidationError, ValidationIssue},
    messages::MessagesProvider,
    rule::RuleContext,
    schema::Schema,
};

/// Wraps an inner schema, applying a mapping function to transform validated `In` into `Out`.
#[derive(Clone)]
pub struct TransformSchema<S, In, Out, F> {
    inner: S,
    transformer: Arc<F>,
    phantom_in: std::marker::PhantomData<In>,
    phantom_out: std::marker::PhantomData<Out>,
}

impl<S, In, Out, F> TransformSchema<S, In, Out, F>
where
    S: Schema<In>,
    F: Fn(In) -> Result<Out, String> + Send + Sync + 'static,
{
    /// Creates a new `TransformSchema`.
    #[must_use]
    pub fn new(inner: S, transformer: F) -> Self {
        Self {
            inner,
            transformer: Arc::new(transformer),
            phantom_in: std::marker::PhantomData,
            phantom_out: std::marker::PhantomData,
        }
    }
}

impl<S, In, Out, F> Schema<Out> for TransformSchema<S, In, Out, F>
where
    S: Schema<In>,
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    F: Fn(In) -> Result<Out, String> + Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<Out, ValidationError> {
        let input = self
            .inner
            .validate_path(value, path, context, parent_data, messages)?;

        match (self.transformer)(input) {
            Ok(transformed) => Ok(transformed),
            Err(err_msg) => {
                let issue = ValidationIssue::new(path, "transform", err_msg);
                Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ))
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
    ) -> BoxFuture<'a, Result<Out, ValidationError>> {
        Box::pin(async move {
            let input = self
                .inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await?;

            match (self.transformer)(input) {
                Ok(transformed) => Ok(transformed),
                Err(err_msg) => {
                    let issue = ValidationIssue::new(path, "transform", err_msg);
                    Err(ValidationError::single(
                        issue.field,
                        issue.rule,
                        issue.message,
                    ))
                }
            }
        })
    }
}

/// Wraps an inner schema with an additional refinement predicate closure.
#[derive(Clone)]
pub struct RefineSchema<S, T, F> {
    inner: S,
    predicate: Arc<F>,
    rule_name: String,
    message: String,
    phantom: std::marker::PhantomData<T>,
}

impl<S, T, F> RefineSchema<S, T, F>
where
    S: Schema<T>,
    F: Fn(&T) -> bool + Send + Sync + 'static,
{
    /// Creates a new `RefineSchema`.
    #[must_use]
    pub fn new(
        inner: S,
        predicate: F,
        rule_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            inner,
            predicate: Arc::new(predicate),
            rule_name: rule_name.into(),
            message: message.into(),
            phantom: std::marker::PhantomData,
        }
    }
    /// Sets a custom rule name.
    #[must_use]
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule_name = rule.into();
        self
    }

    /// Sets a custom error message.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

impl<S, T, F> Schema<T> for RefineSchema<S, T, F>
where
    S: Schema<T>,
    T: Send + Sync + 'static,
    F: Fn(&T) -> bool + Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<T, ValidationError> {
        let valid_val = self
            .inner
            .validate_path(value, path, context, parent_data, messages)?;

        if !(self.predicate)(&valid_val) {
            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let issue =
                rule_ctx.create_issue(&self.rule_name, RuleParams::new(), Some(&self.message));
            return Err(ValidationError::single(
                issue.field,
                issue.rule,
                issue.message,
            ));
        }

        Ok(valid_val)
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
            let valid_val = self
                .inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await?;

            if !(self.predicate)(&valid_val) {
                let rule_ctx = RuleContext::new(path, context, parent_data, messages);
                let issue =
                    rule_ctx.create_issue(&self.rule_name, RuleParams::new(), Some(&self.message));
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }

            Ok(valid_val)
        })
    }
}

/// Wraps an inner schema, applying an infallible mapper function to transform validated `In` into `Out`.
#[derive(Clone)]
pub struct MapSchema<S, In, Out, F> {
    inner: S,
    mapper: Arc<F>,
    phantom_in: std::marker::PhantomData<In>,
    phantom_out: std::marker::PhantomData<Out>,
}

impl<S, In, Out, F> MapSchema<S, In, Out, F>
where
    S: Schema<In>,
    F: Fn(In) -> Out + Send + Sync + 'static,
{
    /// Creates a new `MapSchema`.
    #[must_use]
    pub fn new(inner: S, mapper: F) -> Self {
        Self {
            inner,
            mapper: Arc::new(mapper),
            phantom_in: std::marker::PhantomData,
            phantom_out: std::marker::PhantomData,
        }
    }
}

impl<S, In, Out, F> Schema<Out> for MapSchema<S, In, Out, F>
where
    S: Schema<In>,
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
    F: Fn(In) -> Out + Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<Out, ValidationError> {
        let input = self
            .inner
            .validate_path(value, path, context, parent_data, messages)?;
        Ok((self.mapper)(input))
    }

    #[cfg(feature = "async")]
    fn validate_async_path<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<Out, ValidationError>> {
        Box::pin(async move {
            let input = self
                .inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await?;
            Ok((self.mapper)(input))
        })
    }
}
