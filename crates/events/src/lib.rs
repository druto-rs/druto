//! Strongly typed, asynchronous pub/sub event dispatcher and testing fakes for Druto.
//!
//! # Overview
//!
//! `events` provides an in-process, strongly typed asynchronous event dispatcher. It features
//! zero-copy reference dispatching, wildcard topic routing, short-circuiting execution (`emit_until`),
//! multiple error resolution strategies, and built-in test isolation doubles (`EventFake`).

pub mod concurrent;
pub mod dispatcher;
pub mod error;
pub mod event;
pub mod fake;
pub mod listener;
pub mod topic;

pub use concurrent::ConcurrentFutures;
pub use dispatcher::EventDispatcher;
pub use error::{ErrorStrategy, EventError};
pub use event::{BoxFuture, Event};
pub use fake::{DispatchedEvent, EventFake, FakeFilter};
pub use listener::{
    InfallibleFnListener, IntoListener, Listener, ListenerId, TopicListener, UntilListener,
    from_fn, from_infallible_fn, from_topic_fn, from_until_fn,
};
pub use topic::matches_topic;

#[cfg(test)]
pub(crate) mod test_support {
    use std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    };

    pub(crate) fn noop_waker() -> Waker {
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

    pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
        let mut pinned = Pin::from(Box::new(future));
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);
        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test future yielded Pending unexpectedly"),
        }
    }
}

#[cfg(test)]
pub(crate) use test_support::{block_on, noop_waker};

/// Convenient prelude re-exporting the primary traits, structs, and error types.
pub mod prelude {
    pub use crate::{
        BoxFuture, ErrorStrategy, Event, EventDispatcher, EventError, EventFake,
        InfallibleFnListener, IntoListener, Listener, ListenerId, TopicListener, UntilListener,
        from_fn, from_infallible_fn, from_topic_fn, from_until_fn, matches_topic,
    };
}
