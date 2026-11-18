//! Middleware trait definitions and execution contracts.

use std::{future::Future, pin::Pin, sync::Arc};

use crate::{error::PipelineError, next::Next};

/// A type alias for a pinned, heap-allocated future that is thread-safe across executors.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The asynchronous onion middleware contract.
///
/// Middleware executes pre-processing logic, yields control downstream by calling
/// `next.run(ctx).await`, and executes post-processing logic once downstream execution completes.
pub trait Middleware<Context, Output, Error>: Send + Sync + 'static {
    /// Executes the middleware logic against the mutable context and advances downstream.
    fn handle<'a>(
        &'a self,
        ctx: &'a mut Context,
        next: Next<Context, Output, Error>,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>;
}

/// A thread-safe reference-counted middleware instance.
pub type BoxedMiddleware<Context, Output, Error> = Arc<dyn Middleware<Context, Output, Error>>;

/// An immutable, reference-counted sequence of middleware layers.
pub type MiddlewareStack<Context, Output, Error> = Arc<[BoxedMiddleware<Context, Output, Error>]>;

impl<Context, Output, Error, F> Middleware<Context, Output, Error> for F
where
    Context: Send + 'static,
    Output: Send + 'static,
    Error: Send + 'static,
    F: for<'a> Fn(
            &'a mut Context,
            Next<Context, Output, Error>,
        ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>
        + Send
        + Sync
        + 'static,
{
    fn handle<'a>(
        &'a self,
        ctx: &'a mut Context,
        next: Next<Context, Output, Error>,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>> {
        self(ctx, next)
    }
}

/// Helper function to construct a middleware from a closure with explicit type inference.
pub fn from_fn<Context, Output, Error, F>(middleware: F) -> F
where
    Context: Send + 'static,
    Output: Send + 'static,
    Error: Send + 'static,
    F: for<'a> Fn(
            &'a mut Context,
            Next<Context, Output, Error>,
        ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>
        + Send
        + Sync
        + 'static,
{
    middleware
}
