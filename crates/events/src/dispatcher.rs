//! Core event dispatcher managing typed listeners, wildcard subscriptions, and execution modes.

use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    ops::ControlFlow,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    concurrent::ConcurrentFutures,
    error::{ErrorStrategy, EventError},
    event::Event,
    fake::{EventFake, FakeFilter, FakeState},
    listener::{
        BoxedDynamicListener, BoxedDynamicUntilListener, BoxedTopicListener, IntoListener,
        ListenerId, TopicListener, TypedListenerAdapter, TypedUntilListenerAdapter, UntilListener,
    },
    topic::matches_topic,
};

type ErrorHandler = Arc<dyn Fn(&str, &EventError) + Send + Sync + 'static>;
type TypedListenerEntry = (ListenerId, BoxedDynamicListener);
type TopicListenerEntry = (ListenerId, String, BoxedTopicListener);
type UntilListenerEntry = (ListenerId, Box<dyn Any + Send + Sync>);

struct DispatchSnapshot {
    typed: Vec<TypedListenerEntry>,
    topics: Vec<(ListenerId, BoxedTopicListener)>,
    strategy: ErrorStrategy,
    error_handlers: Vec<ErrorHandler>,
}

struct DispatcherInner {
    typed_listeners: HashMap<TypeId, Vec<TypedListenerEntry>>,
    topic_listeners: Vec<TopicListenerEntry>,
    until_listeners: HashMap<TypeId, Vec<UntilListenerEntry>>,
    once_listeners: HashSet<ListenerId>,
    error_strategy: ErrorStrategy,
    error_handlers: Vec<ErrorHandler>,
    fake_state: Option<Arc<RwLock<FakeState>>>,
}

impl DispatcherInner {
    fn new(error_strategy: ErrorStrategy) -> Self {
        Self {
            typed_listeners: HashMap::new(),
            topic_listeners: Vec::new(),
            until_listeners: HashMap::new(),
            once_listeners: HashSet::new(),
            error_strategy,
            error_handlers: Vec::new(),
            fake_state: None,
        }
    }

    fn matched_topic_listeners(&self, topic: &str) -> Vec<(ListenerId, BoxedTopicListener)> {
        self.topic_listeners
            .iter()
            .filter(|(_, pattern, _)| matches_topic(pattern, topic))
            .map(|(id, _, listener)| (*id, Arc::clone(listener)))
            .collect()
    }

    fn prepare_dispatch(
        &mut self,
        type_id: TypeId,
        event_name: &'static str,
        payload: &dyn Any,
    ) -> Option<DispatchSnapshot> {
        if let Some(fake_ref) = &self.fake_state {
            let mut fake = fake_ref.write().expect("fake lock poisoned");
            if fake.should_fake(type_id, event_name) {
                fake.record(type_id, event_name, payload);
                return None;
            }
        }

        let typed = self
            .typed_listeners
            .get(&type_id)
            .cloned()
            .unwrap_or_default();

        let matched_topics = self.matched_topic_listeners(event_name);

        let mut once_to_remove = Vec::new();
        for (id, _) in &typed {
            if self.once_listeners.contains(id) {
                once_to_remove.push(*id);
            }
        }
        for (id, _) in &matched_topics {
            if self.once_listeners.contains(id) {
                once_to_remove.push(*id);
            }
        }

        for id in &once_to_remove {
            self.once_listeners.remove(id);
            if let Some(list) = self.typed_listeners.get_mut(&type_id) {
                list.retain(|(listener_id, _)| listener_id != id);
            }
            self.topic_listeners
                .retain(|(listener_id, _, _)| listener_id != id);
        }

        Some(DispatchSnapshot {
            typed,
            topics: matched_topics,
            strategy: self.error_strategy,
            error_handlers: self.error_handlers.clone(),
        })
    }
}

/// Thread-safe, asynchronous in-process event dispatcher.
#[derive(Clone)]
pub struct EventDispatcher {
    inner: Arc<RwLock<DispatcherInner>>,
    next_id: Arc<AtomicU64>,
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EventDispatcher {
    /// Creates a new event dispatcher with default [`ErrorStrategy::FailFast`].
    pub fn new() -> Self {
        Self::with_error_strategy(ErrorStrategy::FailFast)
    }

    /// Creates a new event dispatcher with the specified error resolution strategy.
    pub fn with_error_strategy(strategy: ErrorStrategy) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DispatcherInner::new(strategy))),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn generate_id(&self) -> ListenerId {
        ListenerId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Sets the error strategy for subsequent event dispatches.
    pub fn set_error_strategy(&self, strategy: ErrorStrategy) {
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.error_strategy = strategy;
    }

    /// Returns the currently active error strategy.
    pub fn error_strategy(&self) -> ErrorStrategy {
        let inner = self.inner.read().expect("dispatcher lock poisoned");
        inner.error_strategy
    }

    /// Registers a global observer hook invoked whenever any listener returns an error.
    pub fn on_error<F>(&self, handler: F)
    where
        F: Fn(&str, &EventError) + Send + Sync + 'static,
    {
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.error_handlers.push(Arc::new(handler));
    }

    /// Registers a strongly typed listener, accepting either an async closure or a [`crate::Listener`] struct.
    pub fn listen<E: Event, L: IntoListener<E>>(&self, listener: L) -> ListenerId {
        let id = self.generate_id();
        let adapter = TypedListenerAdapter::new(listener.into_listener());
        let boxed: BoxedDynamicListener = Arc::new(adapter);

        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner
            .typed_listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push((id, boxed));
        id
    }

    /// Registers a strongly typed asynchronous closure listener with automatic type coercion.
    pub fn listen_fn<E, F>(&self, listener: F) -> ListenerId
    where
        E: Event,
        F: for<'a> Fn(&'a E) -> crate::event::BoxFuture<'a, Result<(), EventError>>
            + Send
            + Sync
            + 'static,
    {
        self.listen(listener)
    }

    /// Registers a wildcard or dynamic topic listener.
    pub fn listen_topic<L: TopicListener>(&self, pattern: &str, listener: L) -> ListenerId {
        let id = self.generate_id();
        let boxed: BoxedTopicListener = Arc::new(listener);

        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.topic_listeners.push((id, pattern.to_string(), boxed));
        id
    }

    /// Registers a wildcard or dynamic topic listener using an asynchronous closure.
    pub fn listen_topic_fn<F>(&self, pattern: &str, listener: F) -> ListenerId
    where
        F: for<'a> Fn(&'a str, &'a dyn Any) -> crate::event::BoxFuture<'a, Result<(), EventError>>
            + Send
            + Sync
            + 'static,
    {
        self.listen_topic(pattern, listener)
    }

    /// Convenience helper to register a wildcard listener for all events across the entire application.
    pub fn on_any<L: TopicListener>(&self, listener: L) -> ListenerId {
        self.listen_topic("**", listener)
    }

    /// Registers a global wildcard listener using an asynchronous closure.
    pub fn on_any_fn<F>(&self, listener: F) -> ListenerId
    where
        F: for<'a> Fn(&'a str, &'a dyn Any) -> crate::event::BoxFuture<'a, Result<(), EventError>>
            + Send
            + Sync
            + 'static,
    {
        self.on_any(listener)
    }

    /// Registers a one-time listener that automatically unbinds after its first execution.
    pub fn once<E: Event, L: IntoListener<E>>(&self, listener: L) -> ListenerId {
        let id = self.listen(listener);
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.once_listeners.insert(id);
        id
    }

    /// Registers a one-time typed listener using an asynchronous closure.
    pub fn once_fn<E, F>(&self, listener: F) -> ListenerId
    where
        E: Event,
        F: for<'a> Fn(&'a E) -> crate::event::BoxFuture<'a, Result<(), EventError>>
            + Send
            + Sync
            + 'static,
    {
        self.once(listener)
    }

    /// Conditionally registers a listener if `condition` is true.
    pub fn listen_if<E: Event, L: IntoListener<E>>(
        &self,
        condition: bool,
        listener: L,
    ) -> Option<ListenerId> {
        if condition {
            Some(self.listen(listener))
        } else {
            None
        }
    }

    /// Conditionally registers a typed listener using an asynchronous closure.
    pub fn listen_if_fn<E, F>(&self, condition: bool, listener: F) -> Option<ListenerId>
    where
        E: Event,
        F: for<'a> Fn(&'a E) -> crate::event::BoxFuture<'a, Result<(), EventError>>
            + Send
            + Sync
            + 'static,
    {
        self.listen_if(condition, listener)
    }

    /// Registers a short-circuiting listener for [`Self::emit_until`].
    pub fn listen_until<E: Event, B: Send + 'static, L: UntilListener<E, B>>(
        &self,
        listener: L,
    ) -> ListenerId {
        let id = self.generate_id();
        let adapter = TypedUntilListenerAdapter::new(listener);
        let boxed: BoxedDynamicUntilListener<B> = Arc::new(adapter);
        let erased_box: Box<dyn Any + Send + Sync> = Box::new(boxed);

        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner
            .until_listeners
            .entry(TypeId::of::<E>())
            .or_default()
            .push((id, erased_box));
        id
    }

    /// Registers a short-circuiting until listener using an asynchronous closure.
    pub fn listen_until_fn<E, B, F>(&self, listener: F) -> ListenerId
    where
        E: Event,
        B: Send + 'static,
        F: for<'a> Fn(&'a E) -> crate::event::BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>>
            + Send
            + Sync
            + 'static,
    {
        self.listen_until(listener)
    }

    /// Unregisters an active listener by its identifier.
    pub fn off(&self, id: ListenerId) -> bool {
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        let mut removed = false;

        for listeners in inner.typed_listeners.values_mut() {
            let initial_len = listeners.len();
            listeners.retain(|(listener_id, _)| *listener_id != id);
            if listeners.len() < initial_len {
                removed = true;
            }
        }

        let initial_topic_len = inner.topic_listeners.len();
        inner
            .topic_listeners
            .retain(|(listener_id, _, _)| *listener_id != id);
        if inner.topic_listeners.len() < initial_topic_len {
            removed = true;
        }

        for listeners in inner.until_listeners.values_mut() {
            let initial_until_len = listeners.len();
            listeners.retain(|(listener_id, _)| *listener_id != id);
            if listeners.len() < initial_until_len {
                removed = true;
            }
        }

        inner.once_listeners.remove(&id);
        removed
    }

    /// Clears all registered listeners for a specific event type.
    pub fn clear_listeners<E: Event>(&self) {
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        let target_type = TypeId::of::<E>();
        inner.typed_listeners.remove(&target_type);
        inner.until_listeners.remove(&target_type);
    }

    /// Clears all registered typed, topic, and until listeners across the dispatcher.
    pub fn clear_all_listeners(&self) {
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.typed_listeners.clear();
        inner.topic_listeners.clear();
        inner.until_listeners.clear();
        inner.once_listeners.clear();
    }

    /// Checks if any listeners are registered for event type `E`.
    pub fn has_listeners<E: Event>(&self) -> bool {
        self.listener_count::<E>() > 0
    }

    /// Returns the number of typed listeners registered for event type `E`.
    pub fn listener_count<E: Event>(&self) -> usize {
        let inner = self.inner.read().expect("dispatcher lock poisoned");
        inner
            .typed_listeners
            .get(&TypeId::of::<E>())
            .map_or(0, |list| list.len())
    }

    /// Returns the total number of all registered listeners across typed and topic categories.
    pub fn total_listener_count(&self) -> usize {
        let inner = self.inner.read().expect("dispatcher lock poisoned");
        let typed_count: usize = inner.typed_listeners.values().map(Vec::len).sum();
        let until_count: usize = inner.until_listeners.values().map(Vec::len).sum();
        typed_count + until_count + inner.topic_listeners.len()
    }

    /// Dispatches an event sequentially to all registered listeners.
    ///
    /// Alias for [`Self::emit_serial`].
    pub async fn emit<E: Event>(&self, event: &E) -> Result<(), EventError> {
        self.emit_serial(event).await
    }

    fn prepare_dispatch<E: Event>(&self, event: &E) -> Option<DispatchSnapshot> {
        let type_id = TypeId::of::<E>();
        let event_name = event.event_name();
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.prepare_dispatch(type_id, event_name, event.as_any())
    }

    /// Dispatches an event serially in strict registration order.
    pub async fn emit_serial<E: Event>(&self, event: &E) -> Result<(), EventError> {
        let event_name = event.event_name();
        let snapshot = match self.prepare_dispatch(event) {
            Some(snap) => snap,
            None => return Ok(()),
        };

        let mut errors = Vec::new();

        for (_, listener) in snapshot.typed {
            if let Err(error) = listener.handle(event.as_any()).await {
                for handler in &snapshot.error_handlers {
                    handler(event_name, &error);
                }
                if snapshot.strategy == ErrorStrategy::FailFast {
                    return Err(error);
                }
                errors.push(error);
            }
        }

        for (_, listener) in snapshot.topics {
            if let Err(error) = listener.handle(event_name, event.as_any()).await {
                for handler in &snapshot.error_handlers {
                    handler(event_name, &error);
                }
                if snapshot.strategy == ErrorStrategy::FailFast {
                    return Err(error);
                }
                errors.push(error);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(EventError::aggregate(errors))
        }
    }

    /// Dispatches an event concurrently across all registered listeners using cooperative polling.
    pub async fn emit_concurrent<E: Event>(&self, event: &E) -> Result<(), EventError> {
        let event_name = event.event_name();
        let snapshot = match self.prepare_dispatch(event) {
            Some(snap) => snap,
            None => return Ok(()),
        };

        let mut futures = Vec::new();
        for (_, listener) in &snapshot.typed {
            futures.push(listener.handle(event.as_any()));
        }
        for (_, listener) in &snapshot.topics {
            futures.push(listener.handle(event_name, event.as_any()));
        }

        let concurrent = ConcurrentFutures::new(futures, snapshot.strategy);
        match concurrent.await {
            Ok(()) => Ok(()),
            Err(error) => {
                for handler in &snapshot.error_handlers {
                    handler(event_name, &error);
                }
                Err(error)
            }
        }
    }

    /// Dispatches an event sequentially, halting immediately if an until listener returns
    /// [`ControlFlow::Break`] and returning the halting value.
    pub async fn emit_until<E: Event, B: Clone + Send + 'static>(
        &self,
        event: &E,
    ) -> Result<Option<B>, EventError> {
        let type_id = TypeId::of::<E>();
        let event_name = event.event_name();

        let (until_snapshot, error_handlers) = {
            let mut inner = self.inner.write().expect("dispatcher lock poisoned");

            if let Some(fake_ref) = &inner.fake_state {
                let mut fake = fake_ref.write().expect("fake lock poisoned");
                if fake.should_fake(type_id, event_name) {
                    fake.record(type_id, event_name, event.as_any());
                    return Ok(None);
                }
            }

            let mut until_list = Vec::new();
            if let Some(entries) = inner.until_listeners.get(&type_id) {
                for (id, any_listener) in entries {
                    if let Some(downcasted) =
                        any_listener.downcast_ref::<BoxedDynamicUntilListener<B>>()
                    {
                        until_list.push((*id, Arc::clone(downcasted)));
                    }
                }
            }

            let mut once_to_remove = Vec::new();
            for (id, _) in &until_list {
                if inner.once_listeners.contains(id) {
                    once_to_remove.push(*id);
                }
            }

            for id in &once_to_remove {
                inner.once_listeners.remove(id);
                if let Some(list) = inner.until_listeners.get_mut(&type_id) {
                    list.retain(|(listener_id, _)| listener_id != id);
                }
            }

            let error_handlers = inner.error_handlers.clone();
            (until_list, error_handlers)
        };

        for (_, listener) in until_snapshot {
            match listener.handle(event.as_any()).await {
                Ok(ControlFlow::Break(val)) => return Ok(Some(val)),
                Ok(ControlFlow::Continue(())) => {}
                Err(error) => {
                    for handler in &error_handlers {
                        handler(event_name, &error);
                    }
                    return Err(error);
                }
            }
        }

        Ok(None)
    }

    /// Dispatches a dynamic string topic and arbitrary payload to matching topic listeners.
    pub async fn emit_topic(&self, topic: &str, payload: &dyn Any) -> Result<(), EventError> {
        let (topic_snapshot, strategy, error_handlers) = {
            let inner = self.inner.read().expect("dispatcher lock poisoned");
            let matched_topics = inner.matched_topic_listeners(topic);
            let strategy = inner.error_strategy;
            let error_handlers = inner.error_handlers.clone();
            (matched_topics, strategy, error_handlers)
        };

        let mut errors = Vec::new();

        for (_, listener) in topic_snapshot {
            if let Err(error) = listener.handle(topic, payload).await {
                for handler in &error_handlers {
                    handler(topic, &error);
                }
                if strategy == ErrorStrategy::FailFast {
                    return Err(error);
                }
                errors.push(error);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(EventError::aggregate(errors))
        }
    }

    /// Swaps the dispatcher into test fake mode for all events, returning an [`EventFake`] double.
    pub fn fake(&self) -> EventFake {
        let state = Arc::new(RwLock::new(FakeState::new(FakeFilter::All)));
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.fake_state = Some(Arc::clone(&state));
        EventFake::new(state, self.clone())
    }

    /// Activates test fake mode selectively for a single concrete event type `E`.
    pub fn fake_only<E: Event>(&self) -> EventFake {
        let mut set = HashSet::new();
        set.insert(TypeId::of::<E>());
        let state = Arc::new(RwLock::new(FakeState::new(FakeFilter::Types(set))));
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.fake_state = Some(Arc::clone(&state));
        EventFake::new(state, self.clone())
    }

    /// Activates test fake mode selectively for events matching specified topic patterns.
    pub fn fake_topics(&self, patterns: &[&str]) -> EventFake {
        let topic_patterns = patterns.iter().map(|item| item.to_string()).collect();
        let state = Arc::new(RwLock::new(FakeState::new(FakeFilter::Topics(
            topic_patterns,
        ))));
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.fake_state = Some(Arc::clone(&state));
        EventFake::new(state, self.clone())
    }

    /// Restores the dispatcher to normal live dispatching mode, disabling any active fake.
    pub fn restore(&self) {
        let mut inner = self.inner.write().expect("dispatcher lock poisoned");
        inner.fake_state = None;
    }

    /// Returns true if the dispatcher is currently operating in test fake mode.
    pub fn is_faking(&self) -> bool {
        let inner = self.inner.read().expect("dispatcher lock poisoned");
        inner.fake_state.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    use super::*;
    use crate::{Listener, block_on};

    struct OrderCreated {
        id: u64,
    }
    impl Event for OrderCreated {
        fn event_name(&self) -> &'static str {
            "order.created"
        }
    }

    struct OrderShipped {
        id: u64,
    }
    impl Event for OrderShipped {
        fn event_name(&self) -> &'static str {
            "order.shipped"
        }
    }

    struct OrderShippedListener {
        call_count: Arc<AtomicUsize>,
    }

    impl Listener<OrderShipped> for OrderShippedListener {
        fn handle<'a>(
            &'a self,
            event: &'a OrderShipped,
        ) -> crate::event::BoxFuture<'a, Result<(), EventError>> {
            let id = event.id;
            let counter = Arc::clone(&self.call_count);
            Box::pin(async move {
                assert_eq!(id, 100);
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }

    struct CountingListener<E> {
        counter: Arc<AtomicUsize>,
        marker: std::marker::PhantomData<fn(E)>,
    }

    impl<E> CountingListener<E> {
        fn new(counter: &Arc<AtomicUsize>) -> Self {
            Self {
                counter: Arc::clone(counter),
                marker: std::marker::PhantomData,
            }
        }
    }

    impl<E: Event> Listener<E> for CountingListener<E> {
        fn handle<'a>(
            &'a self,
            event: &'a E,
        ) -> crate::event::BoxFuture<'a, Result<(), EventError>> {
            let _ = event;
            let counter = Arc::clone(&self.counter);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }

    struct CountingUntilListener<E, B> {
        counter: Arc<AtomicUsize>,
        result: Option<B>,
        marker: std::marker::PhantomData<fn(E)>,
    }

    impl<E, B> CountingUntilListener<E, B> {
        fn new(counter: &Arc<AtomicUsize>, result: Option<B>) -> Self {
            Self {
                counter: Arc::clone(counter),
                result,
                marker: std::marker::PhantomData,
            }
        }
    }

    impl<E: Event, B: Clone + Send + Sync + 'static> UntilListener<E, B>
        for CountingUntilListener<E, B>
    {
        fn handle<'a>(
            &'a self,
            event: &'a E,
        ) -> crate::event::BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>> {
            let _ = event;
            let counter = Arc::clone(&self.counter);
            let result = self.result.clone();
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                match result {
                    Some(val) => Ok(ControlFlow::Break(val)),
                    None => Ok(ControlFlow::Continue(())),
                }
            })
        }
    }

    struct CountingTopicListener {
        counter: Arc<AtomicUsize>,
    }

    impl CountingTopicListener {
        fn new(counter: &Arc<AtomicUsize>) -> Self {
            Self {
                counter: Arc::clone(counter),
            }
        }
    }

    impl TopicListener for CountingTopicListener {
        fn handle<'a>(
            &'a self,
            topic: &'a str,
            payload: &'a dyn Any,
        ) -> crate::event::BoxFuture<'a, Result<(), EventError>> {
            let _ = (topic, payload);
            let counter = Arc::clone(&self.counter);
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }
    }

    #[test]
    fn test_dispatcher_typed_listener_struct_execution() {
        let dispatcher = EventDispatcher::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let listener = OrderShippedListener {
            call_count: Arc::clone(&counter),
        };
        dispatcher.listen(listener);

        assert_eq!(dispatcher.listener_count::<OrderShipped>(), 1);

        let event = OrderShipped { id: 100 };
        block_on(dispatcher.emit(&event)).expect("emit failed");

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_typed_listener_execution() {
        let dispatcher = EventDispatcher::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = Arc::clone(&call_count);
        dispatcher.listen_fn::<OrderCreated, _>(move |event: &OrderCreated| {
            let count = Arc::clone(&count_clone);
            assert_eq!(event.id, 42);
            Box::pin(async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });

        assert_eq!(dispatcher.listener_count::<OrderCreated>(), 1);
        assert!(dispatcher.has_listeners::<OrderCreated>());
        assert_eq!(dispatcher.total_listener_count(), 1);

        let event = OrderCreated { id: 42 };
        block_on(dispatcher.emit(&event)).expect("emit failed");

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_sequential_order_invariant() {
        let dispatcher = EventDispatcher::new();
        let log = Arc::new(RwLock::new(Vec::new()));

        let log1 = Arc::clone(&log);
        dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
            let log_ref = Arc::clone(&log1);
            Box::pin(async move {
                log_ref.write().unwrap().push(1);
                Ok(())
            })
        });

        let log2 = Arc::clone(&log);
        dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
            let log_ref = Arc::clone(&log2);
            Box::pin(async move {
                log_ref.write().unwrap().push(2);
                Ok(())
            })
        });

        let event = OrderCreated { id: 1 };
        block_on(dispatcher.emit_serial(&event)).expect("emit serial failed");

        let entries = log.read().unwrap().clone();
        assert_eq!(entries, vec![1, 2]);
    }

    #[test]
    fn test_dispatcher_concurrent_execution() {
        let dispatcher = EventDispatcher::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        for _ in 0..5 {
            dispatcher.listen(CountingListener::<OrderCreated>::new(&call_count));
        }

        let event = OrderCreated { id: 7 };
        block_on(dispatcher.emit_concurrent(&event)).expect("concurrent emit failed");
        assert_eq!(call_count.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_dispatcher_wildcard_matching() {
        let dispatcher = EventDispatcher::new();
        let wildcard_called = Arc::new(AtomicUsize::new(0));

        let counter = Arc::clone(&wildcard_called);
        dispatcher.listen_topic_fn("order.*", move |topic: &str, payload: &dyn Any| {
            assert!(topic.starts_with("order."));
            assert!(payload.is::<OrderCreated>());
            let cnt = Arc::clone(&counter);
            Box::pin(async move {
                cnt.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });

        let event = OrderCreated { id: 99 };
        block_on(dispatcher.emit(&event)).expect("emit failed");

        assert_eq!(wildcard_called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_on_any() {
        let dispatcher = EventDispatcher::new();
        let any_called = Arc::new(AtomicUsize::new(0));

        let counter = Arc::clone(&any_called);
        dispatcher.on_any_fn(move |topic: &str, _| {
            let cnt = Arc::clone(&counter);
            let is_match = topic == "order.created" || topic == "order.shipped";
            Box::pin(async move {
                if is_match {
                    cnt.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            })
        });

        block_on(dispatcher.emit(&OrderCreated { id: 1 })).expect("emit failed");
        block_on(dispatcher.emit(&OrderShipped { id: 2 })).expect("emit failed");

        assert_eq!(any_called.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_dispatcher_once_listener() {
        let dispatcher = EventDispatcher::new();
        let call_count = Arc::new(AtomicUsize::new(0));

        dispatcher.once(CountingListener::<OrderCreated>::new(&call_count));

        let event = OrderCreated { id: 1 };
        block_on(dispatcher.emit(&event)).expect("first emit failed");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        block_on(dispatcher.emit(&event)).expect("second emit failed");
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_listen_if() {
        let dispatcher = EventDispatcher::new();
        let called = Arc::new(AtomicUsize::new(0));

        let id1 = dispatcher.listen_if(true, CountingListener::<OrderCreated>::new(&called));
        assert!(id1.is_some());

        let id2 = dispatcher.listen_if(false, CountingListener::<OrderCreated>::new(&called));
        assert!(id2.is_none());

        block_on(dispatcher.emit(&OrderCreated { id: 1 })).expect("emit failed");
        assert_eq!(called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_off_and_clear() {
        let dispatcher = EventDispatcher::new();

        let id =
            dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| Box::pin(async { Ok(()) }));
        assert_eq!(dispatcher.listener_count::<OrderCreated>(), 1);

        let removed = dispatcher.off(id);
        assert!(removed);
        assert_eq!(dispatcher.listener_count::<OrderCreated>(), 0);

        dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| Box::pin(async { Ok(()) }));
        assert_eq!(dispatcher.listener_count::<OrderCreated>(), 1);
        dispatcher.clear_listeners::<OrderCreated>();
        assert_eq!(dispatcher.listener_count::<OrderCreated>(), 0);

        dispatcher.listen_topic_fn("order.*", |_, _| Box::pin(async { Ok(()) }));
        assert_eq!(dispatcher.total_listener_count(), 1);
        dispatcher.clear_all_listeners();
        assert_eq!(dispatcher.total_listener_count(), 0);
    }

    #[test]
    fn test_dispatcher_emit_until_short_circuiting() {
        let dispatcher = EventDispatcher::new();
        let second_called = Arc::new(AtomicUsize::new(0));

        dispatcher.listen_until_fn::<OrderCreated, String, _>(|_: &OrderCreated| {
            Box::pin(async { Ok(ControlFlow::Break("halted_early".to_string())) })
        });

        dispatcher.listen_until(CountingUntilListener::<OrderCreated, String>::new(
            &second_called,
            None,
        ));

        let event = OrderCreated { id: 10 };
        let result = block_on(dispatcher.emit_until::<OrderCreated, String>(&event))
            .expect("emit until failed");

        assert_eq!(result, Some("halted_early".to_string()));
        assert_eq!(second_called.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_dispatcher_emit_until_all_continue() {
        let dispatcher = EventDispatcher::new();

        dispatcher.listen_until_fn::<OrderCreated, u32, _>(|_: &OrderCreated| {
            Box::pin(async { Ok(ControlFlow::Continue(())) })
        });

        let event = OrderCreated { id: 10 };
        let result = block_on(dispatcher.emit_until::<OrderCreated, u32>(&event))
            .expect("emit until failed");
        assert_eq!(result, None);
    }

    #[test]
    fn test_dispatcher_emit_topic_dynamic() {
        let dispatcher = EventDispatcher::new();
        let called = Arc::new(AtomicUsize::new(0));

        let c = Arc::clone(&called);
        dispatcher.listen_topic_fn("audit.auth.*", move |topic: &str, payload: &dyn Any| {
            let counter = Arc::clone(&c);
            let username = payload.downcast_ref::<String>().unwrap().clone();
            assert_eq!(topic, "audit.auth.login");
            assert_eq!(username, "alice");
            Box::pin(async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });

        let payload = "alice".to_string();
        block_on(dispatcher.emit_topic("audit.auth.login", &payload)).expect("emit topic failed");
        assert_eq!(called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_error_strategy_fail_fast_and_hook() {
        let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::FailFast);
        let error_caught = Arc::new(AtomicUsize::new(0));

        let ec = Arc::clone(&error_caught);
        dispatcher.on_error(move |event_name: &str, error: &EventError| {
            assert_eq!(event_name, "order.created");
            assert!(error.to_string().contains("listener failed"));
            ec.fetch_add(1, Ordering::SeqCst);
        });

        dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
            Box::pin(async { Err(EventError::custom("listener failed")) })
        });

        let second_called = Arc::new(AtomicUsize::new(0));
        dispatcher.listen(CountingListener::<OrderCreated>::new(&second_called));

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit(&event)).expect_err("expected error");
        assert!(err.to_string().contains("listener failed"));
        assert_eq!(error_caught.load(Ordering::SeqCst), 1);
        assert_eq!(second_called.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_dispatcher_error_strategy_collect() {
        let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::Collect);

        dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
            Box::pin(async { Err(EventError::custom("err1")) })
        });
        dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
            Box::pin(async { Err(EventError::custom("err2")) })
        });

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit(&event)).expect_err("expected aggregate error");
        assert_eq!(err.error_count(), 2);
    }

    #[test]
    fn test_dispatcher_fake_isolation_and_restore() {
        let dispatcher = EventDispatcher::new();
        let live_called = Arc::new(AtomicUsize::new(0));

        dispatcher.listen(CountingListener::<OrderCreated>::new(&live_called));

        {
            let fake = dispatcher.fake();
            assert!(dispatcher.is_faking());

            let event = OrderCreated { id: 50 };
            block_on(dispatcher.emit(&event)).expect("emit failed in fake");

            assert_eq!(live_called.load(Ordering::SeqCst), 0);
            fake.assert_dispatched::<OrderCreated>();
            fake.assert_dispatched_times::<OrderCreated>(1);
            fake.assert_not_dispatched::<OrderShipped>();
            fake.assert_dispatched_topic("order.*");
            fake.assert_not_dispatched_topic("user.*");
            assert_eq!(fake.total_dispatched(), 1);
        }

        assert!(!dispatcher.is_faking());
        block_on(dispatcher.emit(&OrderCreated { id: 51 })).expect("live emit failed");
        assert_eq!(live_called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_selective_faking() {
        let dispatcher = EventDispatcher::new();
        let order_created_live = Arc::new(AtomicUsize::new(0));
        let order_shipped_live = Arc::new(AtomicUsize::new(0));

        dispatcher.listen(CountingListener::<OrderCreated>::new(&order_created_live));
        dispatcher.listen(CountingListener::<OrderShipped>::new(&order_shipped_live));

        let fake = dispatcher.fake_only::<OrderCreated>();

        block_on(dispatcher.emit(&OrderCreated { id: 1 })).expect("emit failed");
        block_on(dispatcher.emit(&OrderShipped { id: 2 })).expect("emit failed");

        assert_eq!(order_created_live.load(Ordering::SeqCst), 0);
        assert_eq!(order_shipped_live.load(Ordering::SeqCst), 1);

        fake.assert_dispatched::<OrderCreated>();
        fake.assert_not_dispatched::<OrderShipped>();
    }

    #[test]
    fn test_dispatcher_nested_emission_reentrancy() {
        let dispatcher = EventDispatcher::new();
        let dispatcher_clone = dispatcher.clone();
        let secondary_called = Arc::new(AtomicUsize::new(0));

        dispatcher.listen(CountingListener::<OrderShipped>::new(&secondary_called));

        dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
            let d = dispatcher_clone.clone();
            Box::pin(async move {
                d.emit(&OrderShipped { id: 2 })
                    .await
                    .expect("nested emit failed");
                Ok(())
            })
        });

        block_on(dispatcher.emit(&OrderCreated { id: 1 })).expect("outer emit failed");
        assert_eq!(secondary_called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_strategy_getter_and_setter() {
        let dispatcher = EventDispatcher::new();
        assert_eq!(dispatcher.error_strategy(), ErrorStrategy::FailFast);

        dispatcher.set_error_strategy(ErrorStrategy::Collect);
        assert_eq!(dispatcher.error_strategy(), ErrorStrategy::Collect);
    }

    #[test]
    fn test_dispatcher_fake_topics_filter() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake_topics(&["order.*"]);

        let event = OrderCreated { id: 10 };
        block_on(dispatcher.emit(&event)).expect("emit failed");

        fake.assert_dispatched_topic("order.created");
    }

    #[test]
    fn test_dispatcher_emit_concurrent_in_fake_mode() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();

        let event = OrderCreated { id: 25 };
        block_on(dispatcher.emit_concurrent(&event)).expect("concurrent fake emit failed");

        fake.assert_dispatched::<OrderCreated>();
    }

    #[test]
    fn test_dispatcher_emit_until_in_fake_mode() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();

        let event = OrderCreated { id: 30 };
        let result = block_on(dispatcher.emit_until::<OrderCreated, String>(&event))
            .expect("emit until in fake failed");

        assert_eq!(result, None);
        fake.assert_dispatched::<OrderCreated>();
    }

    #[test]
    fn test_dispatcher_once_topic_and_until_listeners() {
        let dispatcher = EventDispatcher::new();
        let topic_calls = Arc::new(AtomicUsize::new(0));

        let topic_id = dispatcher.listen_topic("order.*", CountingTopicListener::new(&topic_calls));

        {
            let mut inner = dispatcher.inner.write().unwrap();
            inner.once_listeners.insert(topic_id);
        }

        let event = OrderCreated { id: 1 };
        block_on(dispatcher.emit_serial(&event)).expect("first emit failed");
        assert_eq!(topic_calls.load(Ordering::SeqCst), 1);

        block_on(dispatcher.emit_serial(&event)).expect("second emit failed");
        assert_eq!(topic_calls.load(Ordering::SeqCst), 1);

        let until_calls = Arc::new(AtomicUsize::new(0));
        let until_id = dispatcher.listen_until(CountingUntilListener::<OrderCreated, u32>::new(
            &until_calls,
            None,
        ));

        {
            let mut inner = dispatcher.inner.write().unwrap();
            inner.once_listeners.insert(until_id);
        }

        let result1 = block_on(dispatcher.emit_until::<OrderCreated, u32>(&event)).unwrap();
        assert_eq!(result1, None);
        assert_eq!(until_calls.load(Ordering::SeqCst), 1);

        let result2 = block_on(dispatcher.emit_until::<OrderCreated, u32>(&event)).unwrap();
        assert_eq!(result2, None);
        assert_eq!(until_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_off_until_listener() {
        let dispatcher = EventDispatcher::new();
        let until_id = dispatcher.listen_until_fn::<OrderCreated, u32, _>(|_: &OrderCreated| {
            Box::pin(async { Ok(ControlFlow::Break(42)) })
        });

        let event = OrderCreated { id: 1 };
        let res1 = block_on(dispatcher.emit_until::<OrderCreated, u32>(&event)).unwrap();
        assert_eq!(res1, Some(42));

        let removed = dispatcher.off(until_id);
        assert!(removed);

        let res2 = block_on(dispatcher.emit_until::<OrderCreated, u32>(&event)).unwrap();
        assert_eq!(res2, None);
    }

    #[test]
    fn test_dispatcher_emit_topic_fail_fast_and_collect() {
        let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::FailFast);
        dispatcher.listen_topic_fn("audit.*", |_, _| {
            Box::pin(async { Err(EventError::custom("topic_err1")) })
        });
        dispatcher.listen_topic_fn("audit.*", |_, _| {
            Box::pin(async { Err(EventError::custom("topic_err2")) })
        });

        let payload = 123u32;
        let err1 =
            block_on(dispatcher.emit_topic("audit.log", &payload)).expect_err("expected error");
        assert!(err1.to_string().contains("topic_err1"));

        dispatcher.set_error_strategy(ErrorStrategy::Collect);
        let err2 =
            block_on(dispatcher.emit_topic("audit.log", &payload)).expect_err("expected error");
        assert_eq!(err2.error_count(), 2);
    }

    #[test]
    fn test_dispatcher_concurrent_topic_listener_failure() {
        let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::FailFast);
        dispatcher.listen_topic_fn("order.*", |_, _| {
            Box::pin(async { Err(EventError::custom("concurrent_topic_failure")) })
        });

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit_concurrent(&event)).expect_err("expected error");
        assert!(err.to_string().contains("concurrent_topic_failure"));
    }

    #[test]
    fn test_dispatcher_emit_until_listener_failure() {
        let dispatcher = EventDispatcher::new();
        dispatcher.listen_until_fn::<OrderCreated, String, _>(|_: &OrderCreated| {
            Box::pin(async { Err(EventError::custom("until_listener_error")) })
        });

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit_until::<OrderCreated, String>(&event))
            .expect_err("expected until error");
        assert!(err.to_string().contains("until_listener_error"));
    }

    #[test]
    fn test_dispatcher_default_and_off_topic() {
        let dispatcher = EventDispatcher::default();
        assert!(!dispatcher.has_listeners::<OrderCreated>());

        let topic_id = dispatcher.listen_topic_fn("orders.*", |_, _| Box::pin(async { Ok(()) }));
        assert_eq!(dispatcher.total_listener_count(), 1);

        let removed = dispatcher.off(topic_id);
        assert!(removed);
        assert_eq!(dispatcher.total_listener_count(), 0);
    }

    #[test]
    fn test_dispatcher_serial_topic_listener_failure_in_fail_fast_and_collect() {
        let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::FailFast);
        dispatcher.listen_topic_fn("order.*", |_, _| {
            Box::pin(async { Err(EventError::custom("serial_topic_fail_fast")) })
        });

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit_serial(&event)).expect_err("expected failure");
        assert!(err.to_string().contains("serial_topic_fail_fast"));

        dispatcher.set_error_strategy(ErrorStrategy::Collect);
        dispatcher.listen_topic_fn("order.*", |_, _| {
            Box::pin(async { Err(EventError::custom("serial_topic_collect_2")) })
        });

        let err_collect = block_on(dispatcher.emit_serial(&event)).expect_err("expected aggregate");
        assert_eq!(err_collect.error_count(), 2);
    }

    #[test]
    fn test_dispatcher_concurrent_topic_listener_failure_in_collect() {
        let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::Collect);
        dispatcher.listen_topic_fn("order.*", |_, _| {
            Box::pin(async { Err(EventError::custom("concurrent_err_1")) })
        });
        dispatcher.listen_topic_fn("order.*", |_, _| {
            Box::pin(async { Err(EventError::custom("concurrent_err_2")) })
        });

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit_concurrent(&event)).expect_err("expected aggregate");
        assert_eq!(err.error_count(), 2);
    }

    #[test]
    fn test_dispatcher_listen_if_fn_and_off_missing() {
        let dispatcher = EventDispatcher::new();
        let ran = Arc::new(AtomicBool::new(false));
        let r = Arc::clone(&ran);
        let id_opt = dispatcher.listen_if_fn::<OrderCreated, _>(true, move |_: &OrderCreated| {
            let flag = Arc::clone(&r);
            Box::pin(async move {
                flag.store(true, Ordering::SeqCst);
                Ok(())
            })
        });
        assert!(id_opt.is_some());
        block_on(dispatcher.emit(&OrderCreated { id: 5 })).unwrap();
        assert!(ran.load(Ordering::SeqCst));

        assert!(!dispatcher.off(ListenerId(999999)));
    }

    #[test]
    fn test_dispatcher_on_error_in_concurrent_until_and_topic() {
        let dispatcher = EventDispatcher::new();
        let errors_seen = Arc::new(AtomicUsize::new(0));
        let es = Arc::clone(&errors_seen);
        dispatcher.on_error(move |topic, error| {
            assert!(!topic.is_empty());
            assert!(error.to_string().contains("test_err"));
            es.fetch_add(1, Ordering::SeqCst);
        });

        dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
            Box::pin(async { Err(EventError::custom("test_err_concurrent")) })
        });
        let _ = block_on(dispatcher.emit_concurrent(&OrderCreated { id: 1 }));
        assert_eq!(errors_seen.load(Ordering::SeqCst), 1);

        dispatcher.listen_until_fn::<OrderCreated, u32, _>(|_: &OrderCreated| {
            Box::pin(async { Err(EventError::custom("test_err_until")) })
        });
        let _ = block_on(dispatcher.emit_until::<OrderCreated, u32>(&OrderCreated { id: 2 }));
        assert_eq!(errors_seen.load(Ordering::SeqCst), 2);

        dispatcher.listen_topic_fn("special.*", |_, _| {
            Box::pin(async { Err(EventError::custom("test_err_topic")) })
        });
        let payload = 10u32;
        let _ = block_on(dispatcher.emit_topic("special.alert", &payload));
        assert_eq!(errors_seen.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_dispatcher_emit_until_downcast_mismatch_and_break_val() {
        let dispatcher = EventDispatcher::new();

        dispatcher.listen_until_fn::<OrderCreated, String, _>(|_: &OrderCreated| {
            Box::pin(async { Ok(ControlFlow::Break("string_val".to_string())) })
        });

        let event = OrderCreated { id: 10 };
        let result = block_on(dispatcher.emit_until::<OrderCreated, u32>(&event)).unwrap();
        assert_eq!(result, None);

        let str_result = block_on(dispatcher.emit_until::<OrderCreated, String>(&event)).unwrap();
        assert_eq!(str_result, Some("string_val".to_string()));

        let count = Arc::new(AtomicUsize::new(0));
        let second_dispatcher = EventDispatcher::new();
        second_dispatcher.listen_until(CountingUntilListener::<OrderCreated, u64>::new(
            &count,
            Some(888),
        ));
        let count_res =
            block_on(second_dispatcher.emit_until::<OrderCreated, u64>(&event)).unwrap();
        assert_eq!(count_res, Some(888));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_on_any_non_matching_event() {
        let dispatcher = EventDispatcher::new();
        let matched = Arc::new(AtomicUsize::new(0));
        let m = Arc::clone(&matched);
        dispatcher.on_any_fn(move |topic, _| {
            let counter = Arc::clone(&m);
            let is_match = topic == "order.created";
            Box::pin(async move {
                if is_match {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            })
        });

        block_on(dispatcher.emit(&OrderShipped { id: 1 })).unwrap();
        assert_eq!(matched.load(Ordering::SeqCst), 0);

        block_on(dispatcher.emit(&OrderCreated { id: 1 })).unwrap();
        assert_eq!(matched.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_emit_concurrent_once_listeners() {
        let dispatcher = EventDispatcher::new();
        let count = Arc::new(AtomicUsize::new(0));
        let topic_count = Arc::new(AtomicUsize::new(0));

        let _ = dispatcher.once(CountingListener::<OrderCreated>::new(&count));
        let topic_id = dispatcher.listen_topic("order.*", CountingTopicListener::new(&topic_count));
        {
            let mut inner = dispatcher.inner.write().unwrap();
            inner.once_listeners.insert(topic_id);
        }

        let event = OrderCreated { id: 42 };
        block_on(dispatcher.emit_concurrent(&event)).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(topic_count.load(Ordering::SeqCst), 1);

        block_on(dispatcher.emit_concurrent(&event)).unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(topic_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_dispatcher_serial_topic_listener_error_with_hook() {
        let dispatcher = EventDispatcher::new();
        let error_hook_called = Arc::new(AtomicBool::new(false));
        let eh = Arc::clone(&error_hook_called);
        dispatcher.on_error(move |topic, error| {
            assert_eq!(topic, "order.created");
            assert!(error.to_string().contains("serial_topic_error"));
            eh.store(true, Ordering::SeqCst);
        });

        dispatcher.listen_topic_fn("order.*", |_, _| {
            Box::pin(async { Err(EventError::custom("serial_topic_error")) })
        });

        let event = OrderCreated { id: 1 };
        let err = block_on(dispatcher.emit_serial(&event)).expect_err("expected error");
        assert!(err.to_string().contains("serial_topic_error"));
        assert!(error_hook_called.load(Ordering::SeqCst));
    }
}
