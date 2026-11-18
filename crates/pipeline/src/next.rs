//! Onion execution step management and double-call protection.

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{
    error::PipelineError,
    middleware::{BoxFuture, MiddlewareStack},
};

/// Contract for the terminal handler executed at the center of the pipeline.
pub trait TerminalHandler<Context, Output, Error>: Send + Sync + 'static {
    /// Executes the terminal handler against the mutable context.
    fn call<'a>(
        &'a self,
        ctx: &'a mut Context,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>;
}

/// Contract for intercepting and recovering from errors during pipeline execution.
pub trait ErrorHandler<Context, Output, Error>: Send + Sync + 'static {
    /// Recovers from an error or rethrows it.
    fn handle<'a>(
        &'a self,
        ctx: &'a mut Context,
        error: PipelineError<Error>,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>;
}

impl<Context, Output, Error, F> ErrorHandler<Context, Output, Error> for F
where
    Context: 'static,
    Output: 'static,
    Error: 'static,
    F: for<'a> Fn(
            &'a mut Context,
            PipelineError<Error>,
        ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>>
        + Send
        + Sync
        + 'static,
{
    fn handle<'a>(
        &'a self,
        ctx: &'a mut Context,
        error: PipelineError<Error>,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>> {
        self(ctx, error)
    }
}

/// A handle passed to middleware to advance execution to the next layer in the onion.
pub struct Next<Context, Output, Error> {
    index: usize,
    middlewares: MiddlewareStack<Context, Output, Error>,
    terminal: Arc<dyn TerminalHandler<Context, Output, Error>>,
    error_handler: Option<Arc<dyn ErrorHandler<Context, Output, Error>>>,
    called: Arc<AtomicBool>,
    error_handled: Arc<AtomicBool>,
}

impl<Context, Output, Error> Next<Context, Output, Error>
where
    Context: Send + 'static,
    Output: Send + 'static,
    Error: Send + 'static,
{
    /// Creates a new `Next` handle pointing to the middleware at `index`.
    pub(crate) fn new(
        index: usize,
        middlewares: MiddlewareStack<Context, Output, Error>,
        terminal: Arc<dyn TerminalHandler<Context, Output, Error>>,
    ) -> Self {
        Self {
            index,
            middlewares,
            terminal,
            error_handler: None,
            called: Arc::new(AtomicBool::new(false)),
            error_handled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Creates a new `Next` handle with an error recovery handler configured.
    pub(crate) fn with_error_handler(
        index: usize,
        middlewares: MiddlewareStack<Context, Output, Error>,
        terminal: Arc<dyn TerminalHandler<Context, Output, Error>>,
        error_handler: Option<Arc<dyn ErrorHandler<Context, Output, Error>>>,
        error_handled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            index,
            middlewares,
            terminal,
            error_handler,
            called: Arc::new(AtomicBool::new(false)),
            error_handled,
        }
    }

    /// Advances the pipeline to the next downstream middleware or terminal handler.
    ///
    /// # Errors
    ///
    /// Returns [`PipelineError::NextAlreadyCalled`] if `run()` is invoked more than
    /// once on the same `Next` instance or any of its clones.
    pub fn run<'a>(
        &self,
        ctx: &'a mut Context,
    ) -> BoxFuture<'a, Result<Output, PipelineError<Error>>> {
        // Atomically prevent double invocation across this handle and any clones.
        if self.called.swap(true, Ordering::SeqCst) {
            return Box::pin(async move { Err(PipelineError::NextAlreadyCalled) });
        }

        let index = self.index;
        let middlewares = Arc::clone(&self.middlewares);
        let terminal = Arc::clone(&self.terminal);
        let error_handler = self.error_handler.clone();
        let error_handled = Arc::clone(&self.error_handled);

        Box::pin(async move {
            let result = if index < middlewares.len() {
                let current_middleware = Arc::clone(&middlewares[index]);
                let child_next = Next::with_error_handler(
                    index + 1,
                    middlewares,
                    terminal,
                    error_handler.clone(),
                    Arc::clone(&error_handled),
                );
                current_middleware.handle(ctx, child_next).await
            } else {
                terminal.call(ctx).await
            };

            match result {
                Ok(output) => Ok(output),
                Err(error) => {
                    if let Some(handler) = error_handler {
                        if !error_handled.swap(true, Ordering::SeqCst) {
                            handler.handle(ctx, error).await
                        } else {
                            Err(error)
                        }
                    } else {
                        Err(error)
                    }
                }
            }
        })
    }

    /// Returns the index of the middleware that will run when `run()` is invoked.
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.index
    }

    /// Returns the number of remaining middleware layers before reaching the terminal handler.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.middlewares.len().saturating_sub(self.index)
    }

    /// Returns `true` if this handle will directly invoke the terminal handler.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        self.index >= self.middlewares.len()
    }

    /// Returns `true` if `run()` has already been called on this handle.
    #[must_use]
    pub fn is_called(&self) -> bool {
        self.called.load(Ordering::SeqCst)
    }
}

impl<Context, Output, Error> Clone for Next<Context, Output, Error> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            middlewares: Arc::clone(&self.middlewares),
            terminal: Arc::clone(&self.terminal),
            error_handler: self.error_handler.clone(),
            called: Arc::clone(&self.called),
            error_handled: Arc::clone(&self.error_handled),
        }
    }
}

impl<Context, Output, Error> fmt::Debug for Next<Context, Output, Error> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Next")
            .field("index", &self.index)
            .field(
                "middlewares_remaining",
                &(self.middlewares.len().saturating_sub(self.index)),
            )
            .field("called", &self.called.load(Ordering::Relaxed))
            .field("error_handled", &self.error_handled.load(Ordering::Relaxed))
            .finish()
    }
}
