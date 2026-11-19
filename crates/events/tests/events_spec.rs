//! Integration test suite for the events crate covering typed dispatch,
//! wildcards, short-circuiting, error strategies, and test doubles.

use std::{
    any::Any,
    fmt,
    future::Future,
    ops::ControlFlow,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use events::{
    BoxFuture, ErrorStrategy, Event, EventDispatcher, EventError, Listener, from_infallible_fn,
    matches_topic,
};
use proptest::prelude::*;

// --- Zero-Dependency Async Test Harness ---

fn noop_waker() -> Waker {
    fn noop_clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &NOOP_VTABLE)
    }
    fn noop_action(data: *const ()) {
        let _ = data;
    }
    static NOOP_VTABLE: RawWakerVTable =
        RawWakerVTable::new(noop_clone, noop_action, noop_action, noop_action);

    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP_VTABLE)) }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut pinned = Pin::from(Box::new(future));
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    match pinned.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => {
            panic!("test future unexpectedly yielded Pending without an external driver")
        }
    }
}

// --- Domain Test Events ---

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserRegistered {
    user_id: u64,
    email: String,
}

impl Event for UserRegistered {
    fn event_name(&self) -> &'static str {
        "user.registered"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderCreated {
    order_id: u64,
    total_amount: u64,
}

impl Event for OrderCreated {
    fn event_name(&self) -> &'static str {
        "order.created"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderShipped {
    order_id: u64,
    tracking_number: String,
}

impl Event for OrderShipped {
    fn event_name(&self) -> &'static str {
        "order.shipped"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InvoiceGenerated {
    invoice_id: u64,
}

impl Event for InvoiceGenerated {
    fn event_name(&self) -> &'static str {
        "billing.invoice.generated"
    }
}

#[derive(Debug, Default)]
struct DefaultNamedEvent;

impl Event for DefaultNamedEvent {}

#[derive(Debug)]
struct CustomTestError(&'static str);

impl fmt::Display for CustomTestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "custom test error: {}", self.0)
    }
}

impl std::error::Error for CustomTestError {}

// --- Test Cases: Typed Dispatch & Ordering ---

#[test]
fn test_default_named_event_dispatch() {
    let dispatcher = EventDispatcher::new();
    let called = Arc::new(AtomicBool::new(false));

    let c = Arc::clone(&called);
    dispatcher.listen_fn::<DefaultNamedEvent, _>(move |_: &DefaultNamedEvent| {
        let flag = Arc::clone(&c);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    let event = DefaultNamedEvent;
    block_on(dispatcher.emit(&event)).expect("emit failed");
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_multiple_typed_listeners_receive_event() {
    let dispatcher = EventDispatcher::new();
    let log = Arc::new(Mutex::new(Vec::new()));

    let log1 = Arc::clone(&log);
    dispatcher.listen_fn::<UserRegistered, _>(move |event: &UserRegistered| {
        let l = Arc::clone(&log1);
        let id = event.user_id;
        Box::pin(async move {
            l.lock().unwrap().push(format!("listener_1:user_{id}"));
            Ok(())
        })
    });

    let log2 = Arc::clone(&log);
    dispatcher.listen_fn::<UserRegistered, _>(move |event: &UserRegistered| {
        let l = Arc::clone(&log2);
        let email = event.email.clone();
        Box::pin(async move {
            l.lock().unwrap().push(format!("listener_2:{email}"));
            Ok(())
        })
    });

    let event = UserRegistered {
        user_id: 42,
        email: "alice@example.com".to_string(),
    };

    block_on(dispatcher.emit(&event)).expect("emit failed");

    let entries = log.lock().unwrap().clone();
    assert_eq!(
        entries,
        vec![
            "listener_1:user_42".to_string(),
            "listener_2:alice@example.com".to_string()
        ]
    );
}

#[test]
fn test_sequential_execution_order_invariance() {
    let dispatcher = EventDispatcher::new();
    let execution_order = Arc::new(Mutex::new(Vec::new()));

    for index in 0..10 {
        let order_ref = Arc::clone(&execution_order);
        dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
            let o = Arc::clone(&order_ref);
            Box::pin(async move {
                o.lock().unwrap().push(index);
                Ok(())
            })
        });
    }

    let event = OrderCreated {
        order_id: 1,
        total_amount: 100,
    };
    block_on(dispatcher.emit_serial(&event)).expect("emit serial failed");

    let expected: Vec<usize> = (0..10).collect();
    assert_eq!(*execution_order.lock().unwrap(), expected);
}

#[test]
fn test_concurrent_execution_all_listeners_run() {
    let dispatcher = EventDispatcher::new();
    let counter = Arc::new(AtomicUsize::new(0));

    for _ in 0..50 {
        let cnt = Arc::clone(&counter);
        dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
            let c = Arc::clone(&cnt);
            Box::pin(async move {
                c.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        });
    }

    let event = OrderCreated {
        order_id: 1,
        total_amount: 50,
    };
    block_on(dispatcher.emit_concurrent(&event)).expect("concurrent emit failed");
    assert_eq!(counter.load(Ordering::SeqCst), 50);
}

// --- Test Cases: Struct Listeners ---

struct NotificationService {
    sent_count: Arc<AtomicUsize>,
}

impl Listener<OrderShipped> for NotificationService {
    fn handle<'a>(&'a self, event: &'a OrderShipped) -> BoxFuture<'a, Result<(), EventError>> {
        let counter = Arc::clone(&self.sent_count);
        let tracking = event.tracking_number.clone();
        Box::pin(async move {
            assert_eq!(tracking, "TRACK-123");
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}

#[test]
fn test_struct_listener_registration_via_into_listener() {
    let dispatcher = EventDispatcher::new();
    let sent_count = Arc::new(AtomicUsize::new(0));

    let service = NotificationService {
        sent_count: Arc::clone(&sent_count),
    };
    let listener_id = dispatcher.listen(service);
    assert_eq!(listener_id.value(), 1);

    let event = OrderShipped {
        order_id: 99,
        tracking_number: "TRACK-123".to_string(),
    };
    block_on(dispatcher.emit(&event)).expect("emit failed");

    assert_eq!(sent_count.load(Ordering::SeqCst), 1);
}

// --- Test Cases: Wildcard & Topic Routing ---

#[test]
fn test_wildcard_single_segment_matching() {
    let dispatcher = EventDispatcher::new();
    let matched_topics = Arc::new(Mutex::new(Vec::new()));

    let topics_ref = Arc::clone(&matched_topics);
    dispatcher.listen_topic_fn("order.*", move |topic: &str, _| {
        let t = Arc::clone(&topics_ref);
        let topic_str = topic.to_string();
        Box::pin(async move {
            t.lock().unwrap().push(topic_str);
            Ok(())
        })
    });

    let order_created = OrderCreated {
        order_id: 1,
        total_amount: 10,
    };
    let order_shipped = OrderShipped {
        order_id: 1,
        tracking_number: "TRK".to_string(),
    };
    let invoice = InvoiceGenerated { invoice_id: 1 };

    block_on(dispatcher.emit(&order_created)).expect("emit order_created failed");
    block_on(dispatcher.emit(&order_shipped)).expect("emit order_shipped failed");
    block_on(dispatcher.emit(&invoice)).expect("emit invoice failed");

    let entries = matched_topics.lock().unwrap().clone();
    assert_eq!(
        entries,
        vec!["order.created".to_string(), "order.shipped".to_string()]
    );
}

#[test]
fn test_wildcard_recursive_multi_segment_matching() {
    let dispatcher = EventDispatcher::new();
    let matched = Arc::new(Mutex::new(Vec::new()));

    let m_ref = Arc::clone(&matched);
    dispatcher.listen_topic_fn("billing.**", move |topic: &str, _| {
        let m = Arc::clone(&m_ref);
        let topic_str = topic.to_string();
        Box::pin(async move {
            m.lock().unwrap().push(topic_str);
            Ok(())
        })
    });

    let invoice = InvoiceGenerated { invoice_id: 100 };
    block_on(dispatcher.emit(&invoice)).expect("emit invoice failed");

    let entries = matched.lock().unwrap().clone();
    assert_eq!(entries, vec!["billing.invoice.generated".to_string()]);
}

#[test]
fn test_on_any_wildcard_catches_all_emissions() {
    let dispatcher = EventDispatcher::new();
    let recorded = Arc::new(Mutex::new(Vec::new()));

    let rec = Arc::clone(&recorded);
    dispatcher.on_any_fn(move |topic: &str, payload: &dyn Any| {
        let r = Arc::clone(&rec);
        let topic_str = topic.to_string();
        let is_user = payload.is::<UserRegistered>();
        let is_order = payload.is::<OrderCreated>();
        Box::pin(async move {
            r.lock().unwrap().push((topic_str, is_user || is_order));
            Ok(())
        })
    });

    let user = UserRegistered {
        user_id: 1,
        email: "u@test.com".to_string(),
    };
    let order = OrderCreated {
        order_id: 2,
        total_amount: 20,
    };

    block_on(dispatcher.emit(&user)).expect("emit user failed");
    block_on(dispatcher.emit(&order)).expect("emit order failed");

    let entries = recorded.lock().unwrap().clone();
    assert_eq!(
        entries,
        vec![
            ("user.registered".to_string(), true),
            ("order.created".to_string(), true)
        ]
    );
}

#[test]
fn test_wildcard_payload_dynamic_downcasting() {
    let dispatcher = EventDispatcher::new();
    let downcast_succeeded = Arc::new(AtomicBool::new(false));

    let flag = Arc::clone(&downcast_succeeded);
    dispatcher.listen_topic_fn("user.*", move |_: &str, payload: &dyn Any| {
        let f = Arc::clone(&flag);
        let downcasted = payload.downcast_ref::<UserRegistered>().cloned();
        Box::pin(async move {
            if let Some(user) = downcasted.filter(|item| item.user_id == 500) {
                let _ = user;
                f.store(true, Ordering::SeqCst);
            }
            Ok(())
        })
    });

    let event = UserRegistered {
        user_id: 500,
        email: "downcast@test.com".to_string(),
    };
    block_on(dispatcher.emit(&event)).expect("emit failed");

    assert!(downcast_succeeded.load(Ordering::SeqCst));
}

// --- Test Cases: Short-Circuiting (emit_until) ---

#[test]
fn test_emit_until_halting_on_first_break() {
    let dispatcher = EventDispatcher::new();
    let third_called = Arc::new(AtomicBool::new(false));

    dispatcher.listen_until_fn::<UserRegistered, &'static str, _>(|_: &UserRegistered| {
        Box::pin(async { Ok(ControlFlow::Continue(())) })
    });

    dispatcher.listen_until_fn::<UserRegistered, &'static str, _>(|event: &UserRegistered| {
        let is_blocked = event.user_id == 999;
        Box::pin(async move {
            if is_blocked {
                Ok(ControlFlow::Break("user_blocked"))
            } else {
                Ok(ControlFlow::Continue(()))
            }
        })
    });

    let tc = Arc::clone(&third_called);
    dispatcher.listen_until_fn::<UserRegistered, &'static str, _>(move |_: &UserRegistered| {
        let flag = Arc::clone(&tc);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(ControlFlow::Continue(()))
        })
    });

    let blocked_event = UserRegistered {
        user_id: 999,
        email: "blocked@test.com".to_string(),
    };

    let result = block_on(dispatcher.emit_until::<UserRegistered, &'static str>(&blocked_event))
        .expect("emit_until failed");

    assert_eq!(result, Some("user_blocked"));
    assert!(!third_called.load(Ordering::SeqCst));
}

#[test]
fn test_emit_until_returns_none_when_all_continue() {
    let dispatcher = EventDispatcher::new();

    dispatcher.listen_until_fn::<OrderCreated, u64, _>(|event: &OrderCreated| {
        let total = event.total_amount;
        Box::pin(async move {
            if total > 1000 {
                Ok(ControlFlow::Break(total))
            } else {
                Ok(ControlFlow::Continue(()))
            }
        })
    });

    let normal_order = OrderCreated {
        order_id: 1,
        total_amount: 150,
    };
    let result = block_on(dispatcher.emit_until::<OrderCreated, u64>(&normal_order))
        .expect("emit_until failed");

    assert_eq!(result, None);
}

// --- Test Cases: Error Strategy & Observer Hook ---

#[test]
fn test_error_strategy_fail_fast_aborts_subsequent_listeners() {
    let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::FailFast);
    let second_executed = Arc::new(AtomicBool::new(false));

    dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
        Box::pin(async { Err(EventError::custom("payment gateway unreachable")) })
    });

    let sec = Arc::clone(&second_executed);
    dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
        let s = Arc::clone(&sec);
        Box::pin(async move {
            s.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    let event = OrderCreated {
        order_id: 1,
        total_amount: 10,
    };
    let result = block_on(dispatcher.emit(&event));

    assert!(result.is_err());
    assert!(!second_executed.load(Ordering::SeqCst));
}

#[test]
fn test_error_strategy_collect_executes_all_and_aggregates() {
    let dispatcher = EventDispatcher::with_error_strategy(ErrorStrategy::Collect);
    let third_executed = Arc::new(AtomicBool::new(false));

    dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
        Box::pin(async { Err(EventError::custom("error_one")) })
    });

    dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
        Box::pin(async { Err(EventError::custom("error_two")) })
    });

    let te = Arc::clone(&third_executed);
    dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
        let flag = Arc::clone(&te);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    let event = OrderCreated {
        order_id: 1,
        total_amount: 10,
    };
    let result = block_on(dispatcher.emit(&event));

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error_count(), 2);
    assert!(third_executed.load(Ordering::SeqCst));
}

#[test]
fn test_on_error_hook_invocation() {
    let dispatcher = EventDispatcher::new();
    let captured_errors = Arc::new(Mutex::new(Vec::new()));

    let ce = Arc::clone(&captured_errors);
    dispatcher.on_error(move |topic: &str, error: &EventError| {
        ce.lock()
            .unwrap()
            .push((topic.to_string(), error.to_string()));
    });

    dispatcher.listen_fn::<OrderCreated, _>(|_: &OrderCreated| {
        Box::pin(async {
            Err(EventError::listener_failed(
                "order.created",
                CustomTestError("timeout"),
            ))
        })
    });

    let event = OrderCreated {
        order_id: 1,
        total_amount: 10,
    };
    let _ = block_on(dispatcher.emit(&event));

    let entries = captured_errors.lock().unwrap().clone();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "order.created");
    assert!(entries[0].1.contains("timeout"));
}

// --- Test Cases: Listener Lifecycle (once, off, listen_if) ---

#[test]
fn test_once_listener_fires_only_on_first_emit() {
    let dispatcher = EventDispatcher::new();
    let execution_count = Arc::new(AtomicUsize::new(0));

    let cnt = Arc::clone(&execution_count);
    dispatcher.once_fn::<OrderCreated, _>(move |_: &OrderCreated| {
        let c = Arc::clone(&cnt);
        Box::pin(async move {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    });

    let event = OrderCreated {
        order_id: 1,
        total_amount: 10,
    };

    block_on(dispatcher.emit(&event)).expect("first emit failed");
    assert_eq!(execution_count.load(Ordering::SeqCst), 1);

    block_on(dispatcher.emit(&event)).expect("second emit failed");
    assert_eq!(execution_count.load(Ordering::SeqCst), 1);

    block_on(dispatcher.emit(&event)).expect("third emit failed");
    assert_eq!(execution_count.load(Ordering::SeqCst), 1);
}

#[test]
fn test_off_unregisters_specific_listener() {
    let dispatcher = EventDispatcher::new();
    let called = Arc::new(AtomicBool::new(false));

    let c = Arc::clone(&called);
    let id = dispatcher.listen_fn::<UserRegistered, _>(move |_: &UserRegistered| {
        let flag = Arc::clone(&c);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    assert_eq!(dispatcher.listener_count::<UserRegistered>(), 1);

    let removed = dispatcher.off(id);
    assert!(removed);
    assert_eq!(dispatcher.listener_count::<UserRegistered>(), 0);

    let event = UserRegistered {
        user_id: 1,
        email: "test@example.com".to_string(),
    };
    block_on(dispatcher.emit(&event)).expect("emit failed");

    assert!(!called.load(Ordering::SeqCst));
}

#[test]
fn test_infallible_listener_registration() {
    let dispatcher = EventDispatcher::new();
    let executed = Arc::new(AtomicBool::new(false));

    let ex = Arc::clone(&executed);
    let listener = from_infallible_fn(move |_: &UserRegistered| {
        let flag = Arc::clone(&ex);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
        })
    });
    dispatcher.listen(listener);

    let event = UserRegistered {
        user_id: 7,
        email: "infallible@test.com".to_string(),
    };
    block_on(dispatcher.emit(&event)).expect("emit failed");

    assert!(executed.load(Ordering::SeqCst));
}

// --- Test Cases: Test Double (EventFake) ---

#[test]
fn test_fake_assertions_and_selective_isolation() {
    let dispatcher = EventDispatcher::new();
    let live_user_called = Arc::new(AtomicBool::new(false));
    let live_order_called = Arc::new(AtomicBool::new(false));

    let luc = Arc::clone(&live_user_called);
    dispatcher.listen_fn::<UserRegistered, _>(move |_: &UserRegistered| {
        let flag = Arc::clone(&luc);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    let loc = Arc::clone(&live_order_called);
    dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
        let flag = Arc::clone(&loc);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    // Fake only UserRegistered events.
    let fake = dispatcher.fake_only::<UserRegistered>();

    let user_event = UserRegistered {
        user_id: 1,
        email: "alice@test.com".to_string(),
    };
    let order_event = OrderCreated {
        order_id: 10,
        total_amount: 50,
    };

    block_on(dispatcher.emit(&user_event)).expect("emit user failed");
    block_on(dispatcher.emit(&order_event)).expect("emit order failed");

    // UserRegistered was faked: live handler should NOT run.
    assert!(!live_user_called.load(Ordering::SeqCst));

    // OrderCreated was NOT faked: live handler SHOULD run.
    assert!(live_order_called.load(Ordering::SeqCst));

    // Assertions on the fake.
    fake.assert_dispatched::<UserRegistered>();
    fake.assert_dispatched_times::<UserRegistered>(1);
    fake.assert_not_dispatched::<OrderCreated>();
    fake.assert_dispatched_topic("user.*");
    fake.assert_not_dispatched_topic("billing.*");
    assert_eq!(fake.dispatched_count::<UserRegistered>(), 1);
    assert_eq!(fake.total_dispatched(), 1);

    fake.flush();
    fake.assert_none_dispatched();
}

#[test]
fn test_fake_on_dispatched_field_inspection() {
    let dispatcher = EventDispatcher::new();
    let fake = dispatcher.fake();

    let captured_email = Arc::new(Mutex::new(String::new()));
    let ce = Arc::clone(&captured_email);

    fake.on_dispatched::<UserRegistered, _>(move |event: &UserRegistered| {
        let mut guard = ce.lock().unwrap();
        *guard = event.email.clone();
    });

    let event = UserRegistered {
        user_id: 123,
        email: "inspector@test.com".to_string(),
    };
    block_on(dispatcher.emit(&event)).expect("emit failed");

    assert_eq!(*captured_email.lock().unwrap(), "inspector@test.com");
}

#[test]
fn test_fake_auto_restoration_on_drop() {
    let dispatcher = EventDispatcher::new();
    let live_called = Arc::new(AtomicUsize::new(0));

    let lc = Arc::clone(&live_called);
    dispatcher.listen_fn::<OrderCreated, _>(move |_: &OrderCreated| {
        let c = Arc::clone(&lc);
        Box::pin(async move {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    });

    {
        let fake = dispatcher.fake();
        assert!(dispatcher.is_faking());

        let event = OrderCreated {
            order_id: 1,
            total_amount: 10,
        };
        block_on(dispatcher.emit(&event)).expect("emit in fake failed");

        assert_eq!(live_called.load(Ordering::SeqCst), 0);
        fake.assert_dispatched::<OrderCreated>();
    }

    // Guard was dropped: live execution must resume automatically.
    assert!(!dispatcher.is_faking());

    let event = OrderCreated {
        order_id: 2,
        total_amount: 20,
    };
    block_on(dispatcher.emit(&event)).expect("live emit failed");
    assert_eq!(live_called.load(Ordering::SeqCst), 1);
}

// --- Test Cases: Concurrency & Re-entrancy ---

#[test]
fn test_nested_emission_and_reentrancy_deadlock_freedom() {
    let dispatcher = EventDispatcher::new();
    let secondary_ran = Arc::new(AtomicBool::new(false));

    let sr = Arc::clone(&secondary_ran);
    dispatcher.listen_fn::<OrderShipped, _>(move |_: &OrderShipped| {
        let flag = Arc::clone(&sr);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(())
        })
    });

    let dispatcher_clone = dispatcher.clone();
    dispatcher.listen_fn::<OrderCreated, _>(move |event: &OrderCreated| {
        let disp = dispatcher_clone.clone();
        let order_id = event.order_id;
        Box::pin(async move {
            let shipped = OrderShipped {
                order_id,
                tracking_number: "AUTO-NESTED".to_string(),
            };
            disp.emit(&shipped).await.expect("nested emit failed");
            Ok(())
        })
    });

    let initial = OrderCreated {
        order_id: 777,
        total_amount: 300,
    };
    block_on(dispatcher.emit(&initial)).expect("outer emit failed");

    assert!(secondary_ran.load(Ordering::SeqCst));
}

// --- Proptest: Wildcard Topic Matching Property Tests ---

proptest! {
    #[test]
    fn proptest_exact_match(s in "[a-z]{1,8}(\\.[a-z]{1,8}){0,4}") {
        prop_assert!(matches_topic(&s, &s));
    }

    #[test]
    fn proptest_global_wildcard(s in "[a-z]{1,8}(\\.[a-z]{1,8}){0,4}") {
        prop_assert!(matches_topic("**", &s));
    }

    #[test]
    fn proptest_single_segment_prefix(
        prefix in "[a-z]{1,6}",
        suffix in "[a-z]{1,6}"
    ) {
        let topic = format!("{prefix}.{suffix}");
        let pattern = format!("{prefix}.*");
        prop_assert!(matches_topic(&pattern, &topic));
    }

    #[test]
    fn proptest_recursive_prefix(
        prefix in "[a-z]{1,6}",
        s1 in "[a-z]{1,6}",
        s2 in "[a-z]{1,6}"
    ) {
        let topic = format!("{prefix}.{s1}.{s2}");
        let pattern = format!("{prefix}.**");
        prop_assert!(matches_topic(&pattern, &topic));
    }
}
