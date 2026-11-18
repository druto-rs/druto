//! Composable asynchronous middleware pipeline.

use std::{fmt, sync::Arc};

use crate::{
    error::PipelineError,
    middleware::{BoxFuture, BoxedMiddleware, Middleware, MiddlewareStack},
    next::{ErrorHandler, Next, TerminalHandler},
};

/// Contract for the terminal handler passed to pipeline execution.
pub trait Terminal<Context, Output, Error>: Send + 'static {
    /// Executes the terminal handler against the mutable context.
    fn call(self, ctx: &mut Context) -> BoxFuture<'_, Result<Output, Error>>;
}

impl<Context, Output, Error, F> Terminal<Context, Output, Error> for F
where
    Context: 'static,
    Output: 'static,
    Error: 'static,
    F: for<'a> FnOnce(&'a mut Context) -> BoxFuture<'a, Result<Output, Error>> + Send + 'static,
{
    fn call(self, ctx: &mut Context) -> BoxFuture<'_, Result<Output, Error>> {
        self(ctx)
    }
}

/// Helper function to construct a terminal handler from a closure with explicit type inference.
pub fn from_terminal<Context, Output, Error, F>(terminal: F) -> F
where
    F: for<'a> FnOnce(&'a mut Context) -> BoxFuture<'a, Result<Output, Error>> + Send + 'static,
{
    terminal
}

/// Helper function to construct an error handler from a closure with explicit type inference.
pub fn from_error_handler<Context, Output, Error, F>(handler: F) -> F
where
    F: for<'a> Fn(
            &'a mut Context,
            PipelineError<Error>,
        ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>
        + Send
        + Sync
        + 'static,
{
    handler
}

/// Adapter to allow `Terminal` to act as an internal `TerminalHandler`.
struct FnOnceTerminal<T> {
    handler: std::sync::Mutex<Option<T>>,
}

impl<Context, Output, Error, T> TerminalHandler<Context, Output, Error> for FnOnceTerminal<T>
where
    Context: Send + 'static,
    Output: Send + 'static,
    Error: Send + 'static,
    T: Terminal<Context, Output, Error>,
{
    fn call<'a>(
        &'a self,
        ctx: &'a mut Context,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>> {
        let mut guard = self
            .handler
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(handler) = guard.take() {
            let future = handler.call(ctx);
            Box::pin(async move {
                match future.await {
                    Ok(output) => Ok(output),
                    Err(error) => Err(PipelineError::Execution(error)),
                }
            })
        } else {
            Box::pin(async move { Err(PipelineError::NextAlreadyCalled) })
        }
    }
}

/// An asynchronous onion middleware pipeline runner.
///
/// Middleware stacks can be assembled using the fluent `.pipe(...)` builder and reused
/// concurrently across multiple threads.
pub struct Pipeline<Context, Output, Error> {
    middlewares: MiddlewareStack<Context, Output, Error>,
}

impl<Context, Output, Error> Pipeline<Context, Output, Error>
where
    Context: Send + 'static,
    Output: Send + 'static,
    Error: Send + 'static,
{
    /// Creates an empty middleware pipeline.
    #[must_use]
    pub fn new() -> Self {
        Self {
            middlewares: Arc::from([]),
        }
    }

    /// Appends a middleware to the end of the pipeline stack.
    #[must_use]
    pub fn pipe<M>(self, middleware: M) -> Self
    where
        M: Middleware<Context, Output, Error>,
    {
        let mut list: Vec<BoxedMiddleware<Context, Output, Error>> = self.middlewares.to_vec();
        list.push(Arc::new(middleware));
        Self {
            middlewares: Arc::from(list),
        }
    }

    /// Appends a middleware in-place to the pipeline stack.
    pub fn push<M>(&mut self, middleware: M)
    where
        M: Middleware<Context, Output, Error>,
    {
        let mut list: Vec<BoxedMiddleware<Context, Output, Error>> = self.middlewares.to_vec();
        list.push(Arc::new(middleware));
        self.middlewares = Arc::from(list);
    }

    /// Appends a pre-boxed middleware trait object to the pipeline stack.
    #[must_use]
    pub fn pipe_arc(self, middleware: BoxedMiddleware<Context, Output, Error>) -> Self {
        let mut list: Vec<BoxedMiddleware<Context, Output, Error>> = self.middlewares.to_vec();
        list.push(middleware);
        Self {
            middlewares: Arc::from(list),
        }
    }

    /// Appends a pre-boxed middleware trait object in-place to the pipeline stack.
    pub fn push_arc(&mut self, middleware: BoxedMiddleware<Context, Output, Error>) {
        let mut list: Vec<BoxedMiddleware<Context, Output, Error>> = self.middlewares.to_vec();
        list.push(middleware);
        self.middlewares = Arc::from(list);
    }

    /// Appends an asynchronous closure middleware to the pipeline stack.
    #[must_use]
    pub fn pipe_fn<F>(self, middleware: F) -> Self
    where
        F: for<'a> Fn(
                &'a mut Context,
                Next<Context, Output, Error>,
            ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>
            + Send
            + Sync
            + 'static,
    {
        self.pipe(middleware)
    }

    /// Merges another pipeline's middleware onto the end of this pipeline.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        let mut list: Vec<BoxedMiddleware<Context, Output, Error>> = self.middlewares.to_vec();
        list.extend(other.middlewares.iter().cloned());
        Self {
            middlewares: Arc::from(list),
        }
    }

    /// Returns the number of registered middleware layers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    /// Returns `true` if the pipeline contains no middleware.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// Executes the pipeline with a terminal handler at the center of the onion.
    ///
    /// User errors returned from the terminal handler are automatically lifted into
    /// [`PipelineError::Execution`].
    pub async fn run<T>(
        &self,
        ctx: &mut Context,
        terminal: T,
    ) -> Result<Output, PipelineError<Error>>
    where
        T: Terminal<Context, Output, Error>,
    {
        let terminal_handler = Arc::new(FnOnceTerminal {
            handler: std::sync::Mutex::new(Some(terminal)),
        });

        let root_next = Next::new(0, Arc::clone(&self.middlewares), terminal_handler);
        root_next.run(ctx).await
    }

    /// Executes the pipeline with an error recovery handler.
    ///
    /// When downstream execution returns an error, the error handler is invoked with the mutable
    /// context and the error. If the handler recovers and returns `Ok(output)`, upstream
    /// post-processing middleware continues executing normally.
    pub async fn run_with_error_handler<T, H>(
        &self,
        ctx: &mut Context,
        terminal: T,
        error_handler: H,
    ) -> Result<Output, PipelineError<Error>>
    where
        T: Terminal<Context, Output, Error>,
        H: ErrorHandler<Context, Output, Error>,
    {
        let terminal_handler = Arc::new(FnOnceTerminal {
            handler: std::sync::Mutex::new(Some(terminal)),
        });
        let recovery_handler = Arc::new(error_handler);

        let root_next = Next::with_error_handler(
            0,
            Arc::clone(&self.middlewares),
            terminal_handler,
            Some(recovery_handler),
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
        );
        root_next.run(ctx).await
    }
}

fn noop_terminal<Context, Error>(context: &mut Context) -> BoxFuture<'_, Result<(), Error>> {
    let _ = context;
    Box::pin(async move { Ok(()) })
}

impl<Context: Send + 'static, Error: Send + 'static> Pipeline<Context, (), Error> {
    /// Executes the pipeline when `Output` is `()`, using a default no-op terminal handler.
    pub async fn run_through(&self, ctx: &mut Context) -> Result<(), PipelineError<Error>> {
        self.run(ctx, noop_terminal::<Context, Error>).await
    }

    /// Executes the pipeline when `Output` is `()`, using a default no-op terminal handler and error recovery.
    pub async fn run_through_with_error_handler<H>(
        &self,
        ctx: &mut Context,
        error_handler: H,
    ) -> Result<(), PipelineError<Error>>
    where
        H: ErrorHandler<Context, (), Error>,
    {
        self.run_with_error_handler(ctx, noop_terminal::<Context, Error>, error_handler)
            .await
    }
}

impl<Context, Output, Error> Default for Pipeline<Context, Output, Error>
where
    Context: Send + 'static,
    Output: Send + 'static,
    Error: Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Context, Output, Error> Clone for Pipeline<Context, Output, Error> {
    fn clone(&self) -> Self {
        Self {
            middlewares: Arc::clone(&self.middlewares),
        }
    }
}

impl<Context, Output, Error> fmt::Debug for Pipeline<Context, Output, Error> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Pipeline")
            .field("middleware_count", &self.middlewares.len())
            .finish()
    }
}
