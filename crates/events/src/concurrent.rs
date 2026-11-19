//! Zero-dependency concurrent future combinator for multi-listener dispatch.

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    error::{ErrorStrategy, EventError},
    event::BoxFuture,
};

/// Future that executes multiple boxed listener futures concurrently.
pub struct ConcurrentFutures<'a> {
    futures: Vec<Option<BoxFuture<'a, Result<(), EventError>>>>,
    errors: Vec<EventError>,
    strategy: ErrorStrategy,
    completed: usize,
    total: usize,
}

impl<'a> ConcurrentFutures<'a> {
    /// Creates a new concurrent future executor over a collection of listener futures.
    pub fn new(
        futures: Vec<BoxFuture<'a, Result<(), EventError>>>,
        strategy: ErrorStrategy,
    ) -> Self {
        let total = futures.len();
        let wrapped = futures.into_iter().map(Some).collect();
        Self {
            futures: wrapped,
            errors: Vec::new(),
            strategy,
            completed: 0,
            total,
        }
    }
}

impl<'a> Future for ConcurrentFutures<'a> {
    type Output = Result<(), EventError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().get_mut();

        if this.total == 0 {
            return Poll::Ready(Ok(()));
        }

        for slot in &mut this.futures {
            if let Some(future) = slot {
                match future.as_mut().poll(context) {
                    Poll::Ready(result) => {
                        this.completed += 1;
                        *slot = None;

                        match result {
                            Ok(()) => {}
                            Err(error) => {
                                if this.strategy == ErrorStrategy::FailFast {
                                    return Poll::Ready(Err(error));
                                }
                                this.errors.push(error);
                            }
                        }
                    }
                    Poll::Pending => {}
                }
            }
        }

        if this.completed == this.total {
            if this.errors.is_empty() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Ready(Err(EventError::aggregate(std::mem::take(&mut this.errors))))
            }
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noop_waker;

    #[test]
    fn test_empty_futures() {
        let future = ConcurrentFutures::new(Vec::new(), ErrorStrategy::FailFast);
        let mut pinned = Box::pin(future);
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);

        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(result) => assert!(result.is_ok()),
            Poll::Pending => panic!("expected ready"),
        }
    }

    #[test]
    fn test_successful_concurrent_completion() {
        let f1: BoxFuture<'_, Result<(), EventError>> = Box::pin(async { Ok(()) });
        let f2: BoxFuture<'_, Result<(), EventError>> = Box::pin(async { Ok(()) });

        let future = ConcurrentFutures::new(vec![f1, f2], ErrorStrategy::FailFast);
        let mut pinned = Box::pin(future);
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);

        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(result) => assert!(result.is_ok()),
            Poll::Pending => panic!("expected ready"),
        }
    }

    #[test]
    fn test_fail_fast_concurrency() {
        let f1: BoxFuture<'_, Result<(), EventError>> =
            Box::pin(async { Err(EventError::custom("err1")) });
        let f2: BoxFuture<'_, Result<(), EventError>> = Box::pin(async { Ok(()) });

        let future = ConcurrentFutures::new(vec![f1, f2], ErrorStrategy::FailFast);
        let mut pinned = Box::pin(future);
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);

        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(result) => {
                let error = result.expect_err("expected error");
                assert!(error.to_string().contains("err1"));
            }
            Poll::Pending => panic!("expected ready"),
        }
    }

    #[test]
    fn test_collect_concurrency() {
        let f1: BoxFuture<'_, Result<(), EventError>> =
            Box::pin(async { Err(EventError::custom("err1")) });
        let f2: BoxFuture<'_, Result<(), EventError>> =
            Box::pin(async { Err(EventError::custom("err2")) });

        let future = ConcurrentFutures::new(vec![f1, f2], ErrorStrategy::Collect);
        let mut pinned = Box::pin(future);
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);

        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(result) => {
                let error = result.expect_err("expected aggregate error");
                assert_eq!(error.error_count(), 2);
            }
            Poll::Pending => panic!("expected ready"),
        }
    }

    struct PendingOnceFuture {
        polled: bool,
    }

    impl Future for PendingOnceFuture {
        type Output = Result<(), EventError>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            let _ = context;
            if !self.polled {
                self.polled = true;
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
    }

    #[test]
    fn test_concurrent_pending_then_ready() {
        let f1: BoxFuture<'_, Result<(), EventError>> =
            Box::pin(PendingOnceFuture { polled: false });
        let f2: BoxFuture<'_, Result<(), EventError>> = Box::pin(async { Ok(()) });

        let future = ConcurrentFutures::new(vec![f1, f2], ErrorStrategy::FailFast);
        let mut pinned = Box::pin(future);
        let waker = noop_waker();
        let mut context = Context::from_waker(&waker);

        // First poll: f1 is Pending, f2 is Ready -> ConcurrentFutures is Pending.
        match pinned.as_mut().poll(&mut context) {
            Poll::Pending => {}
            Poll::Ready(_) => panic!("expected pending on first poll"),
        }

        // Second poll: f1 is Ready, f2 is already completed -> ConcurrentFutures is Ready.
        match pinned.as_mut().poll(&mut context) {
            Poll::Ready(result) => assert!(result.is_ok()),
            Poll::Pending => panic!("expected ready on second poll"),
        }
    }
}
