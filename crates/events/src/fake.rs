//! Test double implementation providing in-memory event recording and assertions.

use std::{
    any::{Any, TypeId},
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::{dispatcher::EventDispatcher, event::Event, topic::matches_topic};

/// Metadata record captured when an event is emitted in fake mode.
#[derive(Debug, Clone)]
pub struct DispatchedEvent {
    /// The compile-time [`TypeId`] of the dispatched event.
    pub type_id: TypeId,
    /// The topic or canonical event name.
    pub event_name: &'static str,
    /// The sequential order index in which this event was emitted.
    pub sequence: usize,
}

/// Filter determining which events should be intercepted and recorded by the fake double.
#[derive(Debug, Clone)]
pub enum FakeFilter {
    /// Intercept all emitted events.
    All,
    /// Intercept only events matching specific [`TypeId`] values.
    Types(HashSet<TypeId>),
    /// Intercept only events matching specific topic wildcard patterns.
    Topics(Vec<String>),
}

pub(crate) type EventInspector = Box<dyn Fn(&dyn Any, &'static str) + Send + Sync + 'static>;

pub(crate) struct FakeState {
    pub(crate) filter: FakeFilter,
    pub(crate) events: Vec<DispatchedEvent>,
    pub(crate) inspectors: Vec<EventInspector>,
}

impl FakeState {
    pub(crate) fn new(filter: FakeFilter) -> Self {
        Self {
            filter,
            events: Vec::new(),
            inspectors: Vec::new(),
        }
    }

    pub(crate) fn should_fake(&self, type_id: TypeId, event_name: &str) -> bool {
        match &self.filter {
            FakeFilter::All => true,
            FakeFilter::Types(types) => types.contains(&type_id),
            FakeFilter::Topics(patterns) => patterns
                .iter()
                .any(|pattern| matches_topic(pattern, event_name)),
        }
    }

    pub(crate) fn record(
        &mut self,
        type_id: TypeId,
        event_name: &'static str,
        event_payload: &dyn Any,
    ) {
        let sequence = self.events.len();
        self.events.push(DispatchedEvent {
            type_id,
            event_name,
            sequence,
        });

        for inspector in &self.inspectors {
            inspector(event_payload, event_name);
        }
    }
}

/// Test double that intercepts event dispatches, prevents live listener side effects,
/// and provides assertion methods for test verification.
pub struct EventFake {
    state: Arc<RwLock<FakeState>>,
    dispatcher: EventDispatcher,
}

impl EventFake {
    pub(crate) fn new(state: Arc<RwLock<FakeState>>, dispatcher: EventDispatcher) -> Self {
        Self { state, dispatcher }
    }

    /// Asserts that at least one event of type `E` was dispatched.
    pub fn assert_dispatched<E: Event>(&self) {
        let count = self.dispatched_count::<E>();
        assert!(
            count > 0,
            "expected event of type '{}' to be dispatched, but 0 events were recorded",
            std::any::type_name::<E>()
        );
    }

    /// Asserts that an event of type `E` was dispatched exactly `expected` times.
    pub fn assert_dispatched_times<E: Event>(&self, expected: usize) {
        let actual = self.dispatched_count::<E>();
        assert_eq!(
            actual,
            expected,
            "expected event '{}' to be dispatched {} times, but was dispatched {} times",
            std::any::type_name::<E>(),
            expected,
            actual
        );
    }

    /// Asserts that no events of type `E` were dispatched.
    pub fn assert_not_dispatched<E: Event>(&self) {
        let count = self.dispatched_count::<E>();
        assert_eq!(
            count,
            0,
            "expected event '{}' not to be dispatched, but was dispatched {} times",
            std::any::type_name::<E>(),
            count
        );
    }

    /// Asserts that zero events of any type were dispatched.
    pub fn assert_none_dispatched(&self) {
        let state = self.state.read().expect("fake lock poisoned");
        assert!(
            state.events.is_empty(),
            "expected zero events to be dispatched, but {} events were recorded: {:?}",
            state.events.len(),
            state.events
        );
    }

    /// Returns the number of times event type `E` was dispatched.
    pub fn dispatched_count<E: Event>(&self) -> usize {
        let target_type_id = TypeId::of::<E>();
        let state = self.state.read().expect("fake lock poisoned");
        state
            .events
            .iter()
            .filter(|record| record.type_id == target_type_id)
            .count()
    }

    /// Returns the total number of all events recorded by the fake.
    pub fn total_dispatched(&self) -> usize {
        let state = self.state.read().expect("fake lock poisoned");
        state.events.len()
    }

    /// Asserts that at least one event matching the given topic pattern was dispatched.
    pub fn assert_dispatched_topic(&self, pattern: &str) {
        let state = self.state.read().expect("fake lock poisoned");
        let found = state
            .events
            .iter()
            .any(|record| matches_topic(pattern, record.event_name));
        assert!(
            found,
            "expected an event matching topic pattern '{pattern}' to be dispatched, but none matched"
        );
    }

    /// Asserts that no event matching the given topic pattern was dispatched.
    pub fn assert_not_dispatched_topic(&self, pattern: &str) {
        let state = self.state.read().expect("fake lock poisoned");
        let found = state
            .events
            .iter()
            .any(|record| matches_topic(pattern, record.event_name));
        assert!(
            !found,
            "expected no events matching topic pattern '{pattern}' to be dispatched, but matching events were found"
        );
    }

    /// Returns a list of all canonical topic names recorded by the fake.
    pub fn dispatched_topics(&self) -> Vec<&'static str> {
        let state = self.state.read().expect("fake lock poisoned");
        state
            .events
            .iter()
            .map(|record| record.event_name)
            .collect()
    }

    /// Returns a snapshot of all dispatched event records.
    pub fn dispatched_records(&self) -> Vec<DispatchedEvent> {
        let state = self.state.read().expect("fake lock poisoned");
        state.events.clone()
    }

    /// Registers an inspection callback that receives the concrete event reference
    /// when the event is emitted in fake mode, allowing deep field assertions.
    pub fn on_dispatched<E: Event, F>(&self, inspector: F)
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let mut state = self.state.write().expect("fake lock poisoned");
        let boxed_inspector: EventInspector = Box::new(move |payload: &dyn Any, _| {
            if let Some(concrete) = payload.downcast_ref::<E>() {
                inspector(concrete);
            }
        });
        state.inspectors.push(boxed_inspector);
    }

    /// Clears all currently recorded events from the buffer.
    pub fn flush(&self) {
        let mut state = self.state.write().expect("fake lock poisoned");
        state.events.clear();
    }

    /// Restores the dispatcher to normal live dispatching mode.
    pub fn restore(&self) {
        self.dispatcher.restore();
    }
}

impl Drop for EventFake {
    fn drop(&mut self) {
        self.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    struct OrderPlaced {
        order_id: u64,
    }
    impl Event for OrderPlaced {
        fn event_name(&self) -> &'static str {
            "order.placed"
        }
    }

    struct UserRegistered;
    impl Event for UserRegistered {
        fn event_name(&self) -> &'static str {
            "user.registered"
        }
    }

    #[test]
    fn test_fake_state_all_filter() {
        let user = UserRegistered;
        assert_eq!(user.event_name(), "user.registered");
        let state = FakeState::new(FakeFilter::All);
        assert!(state.should_fake(TypeId::of::<OrderPlaced>(), "order.placed"));
        assert!(state.should_fake(TypeId::of::<UserRegistered>(), "user.registered"));
    }

    #[test]
    fn test_fake_state_selective_type_filter() {
        let mut set = HashSet::new();
        set.insert(TypeId::of::<OrderPlaced>());
        let state = FakeState::new(FakeFilter::Types(set));

        assert!(state.should_fake(TypeId::of::<OrderPlaced>(), "order.placed"));
        assert!(!state.should_fake(TypeId::of::<UserRegistered>(), "user.registered"));
    }

    #[test]
    fn test_fake_state_topic_filter() {
        let state = FakeState::new(FakeFilter::Topics(vec!["order.*".to_string()]));
        assert!(state.should_fake(TypeId::of::<OrderPlaced>(), "order.placed"));
        assert!(!state.should_fake(TypeId::of::<UserRegistered>(), "user.registered"));
    }

    #[test]
    fn test_fake_state_record_and_inspect() {
        let mut state = FakeState::new(FakeFilter::All);
        let captured = Arc::new(RwLock::new(0u64));
        let captured_clone = Arc::clone(&captured);

        state.inspectors.push(Box::new(move |payload: &dyn Any, _| {
            if let Some(order) = payload.downcast_ref::<OrderPlaced>() {
                let mut guard = captured_clone.write().unwrap();
                *guard = order.order_id;
            }
        }));

        let event = OrderPlaced { order_id: 1234 };
        state.record(TypeId::of::<OrderPlaced>(), "order.placed", &event);

        assert_eq!(state.events.len(), 1);
        assert_eq!(state.events[0].event_name, "order.placed");
        assert_eq!(*captured.read().unwrap(), 1234);
    }

    #[test]
    fn test_fake_dispatched_topics_and_records() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();

        let event = OrderPlaced { order_id: 10 };
        block_on(dispatcher.emit(&event)).expect("emit failed");

        let topics = fake.dispatched_topics();
        assert_eq!(topics, vec!["order.placed"]);

        let inspected_orders = Arc::new(RwLock::new(0usize));
        let io = Arc::clone(&inspected_orders);
        fake.on_dispatched::<OrderPlaced, _>(move |order| {
            assert_eq!(order.order_id, 10);
            *io.write().unwrap() += 1;
        });

        // Trigger inspector for matching type
        block_on(dispatcher.emit(&event)).expect("emit failed");
        assert_eq!(*inspected_orders.read().unwrap(), 1);

        // Trigger inspector for non-matching type
        let other_event = UserRegistered;
        block_on(dispatcher.emit(&other_event)).expect("emit failed");
        assert_eq!(*inspected_orders.read().unwrap(), 1);

        let records = fake.dispatched_records();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].event_name, "order.placed");
        assert_eq!(records[0].sequence, 0);
    }

    #[test]
    #[should_panic(expected = "expected event of type")]
    fn test_assert_dispatched_panics_when_missing() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();
        fake.assert_dispatched::<OrderPlaced>();
    }

    #[test]
    #[should_panic(expected = "expected event '")]
    fn test_assert_dispatched_times_panics_on_mismatch() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();
        fake.assert_dispatched_times::<OrderPlaced>(2);
    }

    #[test]
    #[should_panic(expected = "not to be dispatched")]
    fn test_assert_not_dispatched_panics_when_present() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();
        let event = OrderPlaced { order_id: 1 };
        block_on(dispatcher.emit(&event)).expect("emit failed");
        fake.assert_not_dispatched::<OrderPlaced>();
    }

    #[test]
    #[should_panic(expected = "expected zero events to be dispatched")]
    fn test_assert_none_dispatched_panics_when_present() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();
        let event = OrderPlaced { order_id: 1 };
        block_on(dispatcher.emit(&event)).expect("emit failed");
        fake.assert_none_dispatched();
    }

    #[test]
    #[should_panic(expected = "expected an event matching topic pattern")]
    fn test_assert_dispatched_topic_panics_when_none_match() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();
        fake.assert_dispatched_topic("missing.*");
    }

    #[test]
    #[should_panic(expected = "expected no events matching topic pattern")]
    fn test_assert_not_dispatched_topic_panics_when_matched() {
        let dispatcher = EventDispatcher::new();
        let fake = dispatcher.fake();
        let event = OrderPlaced { order_id: 1 };
        block_on(dispatcher.emit(&event)).expect("emit failed");
        fake.assert_not_dispatched_topic("order.*");
    }
}
