//! Listener traits, adapters, identifiers, and closure contracts.

use std::{any::Any, marker::PhantomData, ops::ControlFlow, sync::Arc};

use crate::{
    error::EventError,
    event::{BoxFuture, Event},
};

/// Unique identifier for a registered event listener handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ListenerId(pub(crate) u64);

impl ListenerId {
    /// Returns the raw numeric representation of the identifier.
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// The asynchronous listener contract for strongly typed events.
pub trait Listener<E: Event>: Send + Sync + 'static {
    /// Executes the listener logic against the immutable event reference.
    fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<(), EventError>>;
}

/// Conversion trait allowing both dedicated listener structs and asynchronous closures
/// to be registered via a unified API.
pub trait IntoListener<E: Event>: Send + Sync + 'static {
    /// Concrete listener type resulting from the conversion.
    type Listener: Listener<E>;

    /// Converts the target into an executable listener.
    fn into_listener(self) -> Self::Listener;
}

impl<E: Event, T: Listener<E>> IntoListener<E> for T {
    type Listener = T;

    fn into_listener(self) -> Self::Listener {
        self
    }
}

impl<E, F> Listener<E> for F
where
    E: Event,
    F: for<'a> Fn(&'a E) -> BoxFuture<'a, Result<(), EventError>> + Send + Sync + 'static,
{
    fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<(), EventError>> {
        self(event)
    }
}

/// Helper function guiding closure type inference to satisfy higher-ranked lifetime bounds (`for<'a>`).
pub fn from_fn<E, F>(listener: F) -> F
where
    E: Event,
    F: for<'a> Fn(&'a E) -> BoxFuture<'a, Result<(), EventError>> + Send + Sync + 'static,
{
    listener
}

/// Helper function guiding closure type inference for infallible async listeners.
pub fn from_infallible_fn<E, F>(listener: F) -> InfallibleFnListener<F>
where
    E: Event,
    F: for<'a> Fn(&'a E) -> BoxFuture<'a, ()> + Send + Sync + 'static,
{
    InfallibleFnListener::new(listener)
}

/// Helper function guiding closure type inference for wildcard and topic listeners.
pub fn from_topic_fn<F>(listener: F) -> F
where
    F: for<'a> Fn(&'a str, &'a dyn Any) -> BoxFuture<'a, Result<(), EventError>>
        + Send
        + Sync
        + 'static,
{
    listener
}

/// Helper function guiding closure type inference for short-circuiting until listeners.
pub fn from_until_fn<E, B, F>(listener: F) -> F
where
    E: Event,
    B: Send + 'static,
    F: for<'a> Fn(&'a E) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>>
        + Send
        + Sync
        + 'static,
{
    listener
}
pub struct InfallibleFnListener<F> {
    callback: F,
}

impl<F> InfallibleFnListener<F> {
    /// Creates a new infallible listener wrapper.
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<E, F> Listener<E> for InfallibleFnListener<F>
where
    E: Event,
    F: for<'a> Fn(&'a E) -> BoxFuture<'a, ()> + Send + Sync + 'static,
{
    fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<(), EventError>> {
        let future = (self.callback)(event);
        Box::pin(async move {
            future.await;
            Ok(())
        })
    }
}

/// Asynchronous listener contract for wildcard and dynamic topic subscriptions.
pub trait TopicListener: Send + Sync + 'static {
    /// Executes listener logic against the matched topic name and type-erased payload.
    fn handle<'a>(
        &'a self,
        topic: &'a str,
        payload: &'a dyn Any,
    ) -> BoxFuture<'a, Result<(), EventError>>;
}

impl<F> TopicListener for F
where
    F: for<'a> Fn(&'a str, &'a dyn Any) -> BoxFuture<'a, Result<(), EventError>>
        + Send
        + Sync
        + 'static,
{
    fn handle<'a>(
        &'a self,
        topic: &'a str,
        payload: &'a dyn Any,
    ) -> BoxFuture<'a, Result<(), EventError>> {
        self(topic, payload)
    }
}

/// Asynchronous listener contract for short-circuiting execution (`emit_until`).
pub trait UntilListener<E: Event, B>: Send + Sync + 'static {
    /// Executes the listener, returning a [`ControlFlow`] signal indicating whether to
    /// halt execution early with a value `B` or continue to subsequent listeners.
    fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>>;
}

impl<E, B, F> UntilListener<E, B> for F
where
    E: Event,
    B: Send + 'static,
    F: for<'a> Fn(&'a E) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>>
        + Send
        + Sync
        + 'static,
{
    fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>> {
        self(event)
    }
}

/// Type-erased listener trait used internally by [`crate::EventDispatcher`].
pub(crate) trait DynamicListener: Send + Sync + 'static {
    fn handle<'a>(&'a self, event: &'a dyn Any) -> BoxFuture<'a, Result<(), EventError>>;
}

pub(crate) struct TypedListenerAdapter<E, L> {
    listener: L,
    marker: PhantomData<fn(E)>,
}

impl<E, L> TypedListenerAdapter<E, L> {
    pub(crate) fn new(listener: L) -> Self {
        Self {
            listener,
            marker: PhantomData,
        }
    }
}

impl<E: Event, L: Listener<E>> DynamicListener for TypedListenerAdapter<E, L> {
    fn handle<'a>(&'a self, event: &'a dyn Any) -> BoxFuture<'a, Result<(), EventError>> {
        if let Some(concrete_event) = event.downcast_ref::<E>() {
            self.listener.handle(concrete_event)
        } else {
            Box::pin(async {
                Err(EventError::custom(
                    "internal downcast failure in dynamic listener adapter",
                ))
            })
        }
    }
}

/// Type-erased until listener trait used internally by [`crate::EventDispatcher`].
pub(crate) trait DynamicUntilListener<B>: Send + Sync + 'static {
    fn handle<'a>(
        &'a self,
        event: &'a dyn Any,
    ) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>>;
}

pub(crate) struct TypedUntilListenerAdapter<E, B, L> {
    listener: L,
    marker: PhantomData<fn(E, B)>,
}

impl<E, B, L> TypedUntilListenerAdapter<E, B, L> {
    pub(crate) fn new(listener: L) -> Self {
        Self {
            listener,
            marker: PhantomData,
        }
    }
}

impl<E: Event, B: Send + 'static, L: UntilListener<E, B>> DynamicUntilListener<B>
    for TypedUntilListenerAdapter<E, B, L>
{
    fn handle<'a>(
        &'a self,
        event: &'a dyn Any,
    ) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>> {
        if let Some(concrete_event) = event.downcast_ref::<E>() {
            self.listener.handle(concrete_event)
        } else {
            Box::pin(async {
                Err(EventError::custom(
                    "internal downcast failure in dynamic until listener adapter",
                ))
            })
        }
    }
}

pub(crate) type BoxedDynamicListener = Arc<dyn DynamicListener>;
pub(crate) type BoxedTopicListener = Arc<dyn TopicListener>;
pub(crate) type BoxedDynamicUntilListener<B> = Arc<dyn DynamicUntilListener<B>>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    struct SimpleEvent {
        value: u32,
    }
    impl Event for SimpleEvent {}

    #[test]
    fn test_listener_id_value() {
        let id = ListenerId(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn test_adapter_downcast_success() {
        let adapter = TypedListenerAdapter::new(from_fn(|event: &SimpleEvent| {
            let val = event.value;
            Box::pin(async move {
                assert_eq!(val, 10);
                Ok(())
            })
        }));

        let event = SimpleEvent { value: 10 };
        let any_event: &dyn Any = &event;
        let result = block_on(DynamicListener::handle(&adapter, any_event));
        assert!(result.is_ok());
    }

    #[test]
    fn test_infallible_listener_wrapper() {
        let wrapper = from_infallible_fn(|event: &SimpleEvent| {
            let val = event.value;
            Box::pin(async move {
                assert_eq!(val, 99);
            })
        });

        let event = SimpleEvent { value: 99 };
        let result = block_on(Listener::handle(&wrapper, &event));
        assert!(result.is_ok());
    }

    struct OtherEvent;
    impl Event for OtherEvent {}

    #[test]
    fn test_adapter_downcast_failure() {
        let adapter =
            TypedListenerAdapter::new(from_fn(|_: &SimpleEvent| Box::pin(async { Ok(()) })));

        let other_event = OtherEvent;
        let any_event: &dyn Any = &other_event;
        let result = block_on(DynamicListener::handle(&adapter, any_event));
        let err = result.expect_err("expected downcast failure");
        assert!(err.to_string().contains("downcast failure"));
    }

    #[test]
    fn test_until_adapter_success_and_failure() {
        let adapter = TypedUntilListenerAdapter::new(from_until_fn(|event: &SimpleEvent| {
            let val = event.value;
            Box::pin(async move { Ok(ControlFlow::Break(val)) })
        }));

        let event = SimpleEvent { value: 77 };
        let any_event: &dyn Any = &event;
        let result = block_on(DynamicUntilListener::handle(&adapter, any_event));
        assert_eq!(result.unwrap(), ControlFlow::Break(77));

        let other = OtherEvent;
        let other_any: &dyn Any = &other;
        let fail_result = block_on(DynamicUntilListener::handle(&adapter, other_any));
        assert!(fail_result.is_err());
    }

    #[test]
    fn test_from_topic_fn_helper() {
        let topic_fn = from_topic_fn(|topic: &str, _: &dyn Any| {
            let t = topic.to_string();
            Box::pin(async move {
                assert_eq!(t, "test.topic");
                Ok(())
            })
        });

        let dummy = 42u32;
        let result = block_on(TopicListener::handle(&topic_fn, "test.topic", &dummy));
        assert!(result.is_ok());
    }
}
