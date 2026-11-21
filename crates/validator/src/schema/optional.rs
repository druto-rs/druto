//! Optional, nullable, and default value schema modifiers.

#[cfg(feature = "async")]
use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::{
    context::ValidationContext, error::ValidationError, messages::MessagesProvider, schema::Schema,
};

/// Wraps an inner schema allowing `null` or missing inputs, producing `Option<T>`.
#[derive(Clone)]
pub struct OptionalSchema<S, T> {
    inner: S,
    phantom: std::marker::PhantomData<T>,
}

impl<S, T> OptionalSchema<S, T> {
    /// Creates a new `OptionalSchema`.
    #[must_use]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<S, T> Schema<Option<T>> for OptionalSchema<S, T>
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
    ) -> Result<Option<T>, ValidationError> {
        if value.is_null() {
            return Ok(None);
        }

        self.inner
            .validate_path(value, path, context, parent_data, messages)
            .map(Some)
    }

    #[cfg(feature = "async")]
    fn validate_async_path<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<Option<T>, ValidationError>> {
        Box::pin(async move {
            if value.is_null() {
                return Ok(None);
            }

            self.inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await
                .map(Some)
        })
    }
}

/// Wraps an inner schema allowing explicit JSON `null`, producing `Option<T>`.
#[derive(Clone)]
pub struct NullableSchema<S, T> {
    inner: S,
    phantom: std::marker::PhantomData<T>,
}

impl<S, T> NullableSchema<S, T> {
    /// Creates a new `NullableSchema`.
    #[must_use]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<S, T> Schema<Option<T>> for NullableSchema<S, T>
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
    ) -> Result<Option<T>, ValidationError> {
        if value.is_null() {
            return Ok(None);
        }

        self.inner
            .validate_path(value, path, context, parent_data, messages)
            .map(Some)
    }

    #[cfg(feature = "async")]
    fn validate_async_path<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<Option<T>, ValidationError>> {
        Box::pin(async move {
            if value.is_null() {
                return Ok(None);
            }

            self.inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await
                .map(Some)
        })
    }
}

/// Wraps an inner schema, providing a fallback default value if the input is `null`.
#[derive(Clone)]
pub struct DefaultSchema<S, T> {
    inner: S,
    default_value: T,
}

impl<S, T> DefaultSchema<S, T> {
    /// Creates a new `DefaultSchema`.
    #[must_use]
    pub fn new(inner: S, default_value: T) -> Self {
        Self {
            inner,
            default_value,
        }
    }
}

impl<S, T> Schema<T> for DefaultSchema<S, T>
where
    S: Schema<T>,
    T: Clone + Send + Sync + 'static,
{
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<T, ValidationError> {
        if value.is_null() {
            return Ok(self.default_value.clone());
        }

        self.inner
            .validate_path(value, path, context, parent_data, messages)
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
            if value.is_null() {
                return Ok(self.default_value.clone());
            }

            self.inner
                .validate_async_path(value, path, context, parent_data, messages)
                .await
        })
    }
}
