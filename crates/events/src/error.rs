//! Error types and error handling strategies for event dispatching.

use std::fmt;

/// Strategy to adopt when a listener returns an error during event dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorStrategy {
    /// Stop execution immediately upon encountering the first error and bubble it up.
    #[default]
    FailFast,

    /// Continue invoking subsequent listeners even if errors occur, aggregating all failures.
    Collect,
}

/// Errors that can occur during event registration, listener execution, or dispatch.
#[derive(Debug)]
pub enum EventError {
    /// A specific listener execution failed with an underlying error.
    ListenerFailed {
        /// The canonical name of the event being dispatched.
        event_name: &'static str,
        /// The underlying cause of the failure.
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Multiple listeners failed when dispatching under [`ErrorStrategy::Collect`].
    Aggregate(Vec<EventError>),

    /// Custom error message.
    Custom(String),
}

impl EventError {
    /// Creates a new listener execution error.
    pub fn listener_failed<E>(event_name: &'static str, error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::ListenerFailed {
            event_name,
            source: error.into(),
        }
    }

    /// Creates a custom event error with the provided message.
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }

    /// Creates an aggregate error combining multiple listener errors.
    pub fn aggregate(errors: Vec<Self>) -> Self {
        Self::Aggregate(errors)
    }

    /// Returns the number of errors contained within this error.
    pub fn error_count(&self) -> usize {
        match self {
            Self::Aggregate(errors) => errors.iter().map(Self::error_count).sum(),
            _ => 1,
        }
    }

    /// Returns true if this error contains no underlying errors.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Aggregate(errors) => errors.is_empty(),
            _ => false,
        }
    }

    /// Returns a slice of nested errors if this is an aggregate error.
    pub fn errors(&self) -> &[Self] {
        match self {
            Self::Aggregate(errors) => errors.as_slice(),
            _ => std::slice::from_ref(self),
        }
    }
}

impl fmt::Display for EventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ListenerFailed { event_name, source } => {
                write!(
                    formatter,
                    "listener failed during execution of event '{event_name}': {source}"
                )
            }
            Self::Aggregate(errors) => {
                write!(
                    formatter,
                    "multiple listener errors occurred ({} errors):",
                    errors.len()
                )?;
                for (index, error) in errors.iter().enumerate() {
                    write!(formatter, "\n  [{index}] {error}")?;
                }
                Ok(())
            }
            Self::Custom(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for EventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ListenerFailed { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyError;

    impl fmt::Display for DummyError {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "dummy failure")
        }
    }

    impl std::error::Error for DummyError {}

    #[test]
    fn test_error_strategy_default() {
        assert_eq!(ErrorStrategy::default(), ErrorStrategy::FailFast);
    }

    #[test]
    fn test_listener_failed_formatting_and_source() {
        let error = EventError::listener_failed("order.created", DummyError);
        assert_eq!(error.error_count(), 1);
        assert!(!error.is_empty());

        let formatted = error.to_string();
        assert!(formatted.contains("order.created"));
        assert!(formatted.contains("dummy failure"));

        use std::error::Error;
        assert!(error.source().is_some());
    }

    #[test]
    fn test_aggregate_error_handling() {
        let err1 = EventError::custom("first failure");
        let err2 = EventError::custom("second failure");
        let aggregate = EventError::aggregate(vec![err1, err2]);

        assert_eq!(aggregate.error_count(), 2);
        assert_eq!(aggregate.errors().len(), 2);

        let formatted = aggregate.to_string();
        assert!(formatted.contains("2 errors"));
        assert!(formatted.contains("[0] first failure"));
        assert!(formatted.contains("[1] second failure"));

        let empty_aggregate = EventError::aggregate(Vec::new());
        assert!(empty_aggregate.is_empty());

        let single = EventError::custom("single");
        assert!(!single.is_empty());
        assert_eq!(single.errors().len(), 1);
        use std::error::Error;
        assert!(single.source().is_none());
        assert_eq!(single.to_string(), "single");
    }
}
