//! Asynchronous onion middleware pipeline and execution runner.
//!
//! `pipeline` provides a composable, generic asynchronous chain-of-responsibility (onion)
//! execution engine for the Druto framework. It allows an arbitrary context and request/response
//! lifecycle to be passed through a sequential series of interceptors (middleware) wrapping a
//! terminal handler.

mod error;
mod middleware;
mod next;
mod pipeline;

pub use error::PipelineError;
pub use middleware::{BoxFuture, BoxedMiddleware, Middleware, MiddlewareStack, from_fn};
pub use next::{ErrorHandler, Next, TerminalHandler};
pub use pipeline::{Pipeline, Terminal, from_error_handler, from_terminal};
