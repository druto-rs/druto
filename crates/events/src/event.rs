//! Event trait definition, metadata, and future type aliases.

use std::{any::Any, future::Future, pin::Pin};

/// A pinned, heap-allocated future that is thread-safe across asynchronous executors.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The core event contract associating an event type with an immutable topic name and dynamic upcasting.
pub trait Event: Send + Sync + 'static {
    /// Returns the canonical topic or event name.
    ///
    /// Defaults to the fully qualified Rust type name (`std::any::type_name::<Self>()`).
    /// Implementors can override this method to assign clean, dot-separated topic names
    /// (e.g., `"order.created"` or `"user.registered"`).
    fn event_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Provides dynamic reference upcasting to [`Any`] for wildcard listeners.
    ///
    /// Allows wildcard listeners to dynamically inspect or downcast event payloads
    /// with zero allocations and zero serialization overhead.
    fn as_any(&self) -> &dyn Any
    where
        Self: Sized,
    {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DefaultNamedEvent;
    impl Event for DefaultNamedEvent {}

    struct CustomNamedEvent {
        order_id: u64,
    }
    impl Event for CustomNamedEvent {
        fn event_name(&self) -> &'static str {
            "order.placed"
        }
    }

    #[test]
    fn test_default_event_name() {
        let event = DefaultNamedEvent;
        assert!(event.event_name().contains("DefaultNamedEvent"));
    }

    #[test]
    fn test_custom_event_name() {
        let event = CustomNamedEvent { order_id: 42 };
        assert_eq!(event.event_name(), "order.placed");
    }

    #[test]
    fn test_as_any_downcast() {
        let event = CustomNamedEvent { order_id: 100 };
        let any_ref = event.as_any();
        let downcasted = any_ref.downcast_ref::<CustomNamedEvent>();
        assert!(downcasted.is_some());
        assert_eq!(downcasted.unwrap().order_id, 100);
    }
}
