//! Server error types, protocol failures, and listener exceptions.

use std::{fmt, io};

/// Errors encountered during server configuration, listener binding, or request execution.
#[derive(Debug)]
pub enum ServerError {
    /// Underlying asynchronous I/O error on TCP or Unix sockets.
    Io(io::Error),
    /// HTTP protocol or message framing violation produced by the underlying server engine.
    Protocol(sebok::SebokError),
    /// Routing compilation or dispatch failure.
    Routing(router::RouterError),
    /// Socket address or host resolution failure.
    AddressParse(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "Server I/O error: {err}"),
            Self::Protocol(err) => write!(formatter, "HTTP engine error: {err}"),
            Self::Routing(err) => write!(formatter, "Router error: {err}"),
            Self::AddressParse(msg) => write!(formatter, "Address resolution failure: {msg}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Protocol(err) => Some(err),
            Self::Routing(err) => Some(err),
            Self::AddressParse(_) => None,
        }
    }
}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<sebok::SebokError> for ServerError {
    fn from(err: sebok::SebokError) -> Self {
        Self::Protocol(err)
    }
}

impl From<router::RouterError> for ServerError {
    fn from(err: router::RouterError) -> Self {
        Self::Routing(err)
    }
}
