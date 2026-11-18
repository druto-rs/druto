//! Error types for asynchronous onion pipeline execution.

use std::fmt;

/// Errors produced during middleware pipeline execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineError<E> {
    /// Next handle was called more than once within the same middleware layer.
    NextAlreadyCalled,
    /// An application error returned by downstream middleware or the terminal handler.
    Execution(E),
}

impl<E: fmt::Display> fmt::Display for PipelineError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NextAlreadyCalled => {
                write!(
                    formatter,
                    "pipeline step next.run() was invoked more than once"
                )
            }
            Self::Execution(error) => write!(formatter, "{error}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for PipelineError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NextAlreadyCalled => None,
            Self::Execution(error) => Some(error),
        }
    }
}

impl<E> From<E> for PipelineError<E> {
    fn from(error: E) -> Self {
        Self::Execution(error)
    }
}

impl<E> PipelineError<E> {
    /// Returns `true` if this error represents an illegal secondary `next.run()` call.
    #[must_use]
    pub fn is_next_already_called(&self) -> bool {
        matches!(self, Self::NextAlreadyCalled)
    }

    /// Returns `true` if this error represents an application execution failure.
    #[must_use]
    pub fn is_execution(&self) -> bool {
        matches!(self, Self::Execution(_))
    }

    /// Returns a reference to the inner application error if present.
    #[must_use]
    pub fn execution(&self) -> Option<&E> {
        match self {
            Self::Execution(error) => Some(error),
            Self::NextAlreadyCalled => None,
        }
    }

    /// Returns a mutable reference to the inner application error if present.
    #[must_use]
    pub fn execution_mut(&mut self) -> Option<&mut E> {
        match self {
            Self::Execution(error) => Some(error),
            Self::NextAlreadyCalled => None,
        }
    }

    /// Consumes the error, returning the inner application error if present.
    #[must_use]
    pub fn into_execution(self) -> Option<E> {
        match self {
            Self::Execution(error) => Some(error),
            Self::NextAlreadyCalled => None,
        }
    }

    /// Transforms the inner application error type using the given function.
    pub fn map_execution<Target, F: FnOnce(E) -> Target>(
        self,
        transform: F,
    ) -> PipelineError<Target> {
        match self {
            Self::NextAlreadyCalled => PipelineError::NextAlreadyCalled,
            Self::Execution(error) => PipelineError::Execution(transform(error)),
        }
    }
}
