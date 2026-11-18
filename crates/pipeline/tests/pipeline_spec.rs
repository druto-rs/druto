//! Comprehensive test suite for the pipeline crate.

use std::{
    collections::HashMap,
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use pipeline::{
    BoxFuture, Middleware, Next, Pipeline, PipelineError, from_error_handler, from_fn,
    from_terminal,
};

// --- Test Utilities (Zero-Dependency Executor) ---

fn noop_waker() -> Waker {
    fn noop_clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &NOOP_VTABLE)
    }
    fn noop_action(data: *const ()) {
        let _ = data;
    }
    static NOOP_VTABLE: RawWakerVTable =
        RawWakerVTable::new(noop_clone, noop_action, noop_action, noop_action);

    // SAFETY: RawWaker implementation performs no operations and maintains valid null pointer state.
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

// --- Domain Test Types ---

#[derive(Debug, Default, Clone)]
struct TestContext {
    log: Vec<String>,
    headers: HashMap<String, String>,
    authenticated: bool,
    counter: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AppError {
    Unauthorized,
    NotFound(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized => write!(formatter, "unauthorized access"),
            Self::NotFound(resource) => write!(formatter, "not found: {resource}"),
            Self::Internal(detail) => write!(formatter, "internal error: {detail}"),
        }
    }
}

impl std::error::Error for AppError {}

// --- Structured Middleware Implementations ---

struct TrackingMiddleware {
    name: &'static str,
}

impl Middleware<TestContext, String, AppError> for TrackingMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut TestContext,
        next: Next<TestContext, String, AppError>,
    ) -> BoxFuture<'a, Result<String, PipelineError<AppError>>> {
        Box::pin(async move {
            ctx.log.push(format!("{}:start", self.name));
            let output = next.run(ctx).await?;
            ctx.log.push(format!("{}:end", self.name));
            Ok(output)
        })
    }
}

struct AuthMiddleware;

impl Middleware<TestContext, String, AppError> for AuthMiddleware {
    fn handle<'a>(
        &'a self,
        ctx: &'a mut TestContext,
        next: Next<TestContext, String, AppError>,
    ) -> BoxFuture<'a, Result<String, PipelineError<AppError>>> {
        Box::pin(async move {
            if !ctx.authenticated {
                // Short-circuit downstream execution.
                return Ok("blocked_by_auth".to_string());
            }
            next.run(ctx).await
        })
    }
}

// --- Test Cases ---

#[test]
fn test_execution_order_strict_onion_model() {
    let pipeline = Pipeline::new()
        .pipe(TrackingMiddleware { name: "mw1" })
        .pipe(TrackingMiddleware { name: "mw2" })
        .pipe(TrackingMiddleware { name: "mw3" });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            Box::pin(async move {
                target_ctx.log.push("handler:exec".to_string());
                Ok("success".to_string())
            })
        }),
    ));

    assert_eq!(result, Ok("success".to_string()));
    assert_eq!(
        ctx.log,
        vec![
            "mw1:start",
            "mw2:start",
            "mw3:start",
            "handler:exec",
            "mw3:end",
            "mw2:end",
            "mw1:end",
        ]
    );
}

#[test]
fn test_short_circuit_halts_downstream_and_unwinds_upstream() {
    let pipeline = Pipeline::new()
        .pipe(TrackingMiddleware { name: "mw1" })
        .pipe(AuthMiddleware)
        .pipe(TrackingMiddleware { name: "mw2" });

    let mut ctx = TestContext {
        authenticated: false,
        ..Default::default()
    };

    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            Box::pin(async move {
                target_ctx.log.push("handler:exec".to_string());
                Ok("handler_output".to_string())
            })
        }),
    ));

    assert_eq!(result, Ok("blocked_by_auth".to_string()));
    // mw2 and handler must never execute, but mw1 should finish post-processing.
    assert_eq!(ctx.log, vec!["mw1:start", "mw1:end"]);
}

#[test]
fn test_double_next_call_fails_predictably() {
    let pipeline = Pipeline::new().pipe_fn(
        |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
            Box::pin(async move {
                let first_call = next.run(ctx).await;
                assert!(first_call.is_ok());

                // Second call on the same handle must return NextAlreadyCalled.
                let second_call = next.run(ctx).await;
                assert_eq!(second_call, Err(PipelineError::NextAlreadyCalled));

                first_call
            })
        },
    );

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            Box::pin(async move {
                target_ctx.counter += 1;
                Ok("called".to_string())
            })
        }),
    ));

    assert_eq!(result, Ok("called".to_string()));
    // Terminal handler must only have been invoked once despite the double call.
    assert_eq!(ctx.counter, 1);
}

#[test]
fn test_double_next_call_across_clones_fails_predictably() {
    let pipeline = Pipeline::new().pipe_fn(
        |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
            Box::pin(async move {
                let next_clone = next.clone();
                let first_call = next.run(ctx).await;
                assert!(first_call.is_ok());

                // Invoking on a clone must also observe that the step was already executed.
                let second_call = next_clone.run(ctx).await;
                assert_eq!(second_call, Err(PipelineError::NextAlreadyCalled));

                first_call
            })
        },
    );

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            Box::pin(async move {
                target_ctx.counter += 1;
                Ok("done".to_string())
            })
        }),
    ));

    assert_eq!(result, Ok("done".to_string()));
    assert_eq!(ctx.counter, 1);
}

#[test]
fn test_error_propagation_and_upstream_recovery() {
    let pipeline = Pipeline::new()
        .pipe_fn(
            |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
                Box::pin(async move {
                    ctx.log.push("outer:before".to_string());
                    match next.run(ctx).await {
                        Ok(output) => Ok(output),
                        Err(PipelineError::Execution(AppError::NotFound(resource))) => {
                            ctx.log.push(format!("caught:{resource}"));
                            Ok("recovered_output".to_string())
                        }
                        Err(error) => Err(error),
                    }
                })
            },
        )
        .pipe(TrackingMiddleware { name: "inner" });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Err(AppError::NotFound("users/42".to_string())) })
        }),
    ));

    assert_eq!(result, Ok("recovered_output".to_string()));
    assert_eq!(
        ctx.log,
        vec!["outer:before", "inner:start", "caught:users/42"]
    );
}

#[test]
fn test_pipeline_run_with_error_handler_recovers_gracefully() {
    let pipeline = Pipeline::new()
        .pipe(TrackingMiddleware { name: "logger" })
        .pipe(TrackingMiddleware { name: "profiler" });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run_with_error_handler(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Err(AppError::Internal("db timeout".to_string())) })
        }),
        from_error_handler(
            |error_ctx: &mut TestContext, error: PipelineError<AppError>| {
                Box::pin(async move {
                    error_ctx.log.push("error_handler:recovered".to_string());
                    assert_eq!(
                        error,
                        PipelineError::Execution(AppError::Internal("db timeout".to_string()))
                    );
                    Ok("fallback_response".to_string())
                })
            },
        ),
    ));

    assert_eq!(result, Ok("fallback_response".to_string()));
    assert_eq!(
        ctx.log,
        vec![
            "logger:start",
            "profiler:start",
            "error_handler:recovered",
            "profiler:end",
            "logger:end",
        ]
    );
}

#[test]
fn test_unhandled_error_without_recovery_bubbles_up() {
    let pipeline = Pipeline::new().pipe(TrackingMiddleware { name: "logger" });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Err(AppError::Unauthorized) })
        }),
    ));

    assert_eq!(
        result,
        Err(PipelineError::Execution(AppError::Unauthorized))
    );
    // Logger started, but because error was not caught, post-processing did not run.
    assert_eq!(ctx.log, vec!["logger:start"]);
}

#[test]
fn test_closure_middleware_via_pipe_and_pipe_fn() {
    let pipeline = Pipeline::new()
        .pipe_fn(
            |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
                Box::pin(async move {
                    ctx.headers.insert("X-Custom".to_string(), "1".to_string());
                    next.run(ctx).await
                })
            },
        )
        .pipe_fn(
            |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
                Box::pin(async move {
                    ctx.headers.insert("X-Second".to_string(), "2".to_string());
                    next.run(ctx).await
                })
            },
        );

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("ok".to_string()) })
        }),
    ));

    assert_eq!(result, Ok("ok".to_string()));
    assert_eq!(ctx.headers.get("X-Custom").map(|s| s.as_str()), Some("1"));
    assert_eq!(ctx.headers.get("X-Second").map(|s| s.as_str()), Some("2"));
}

#[test]
fn test_pipeline_merge_and_clone_branching() {
    let base_pipeline = Pipeline::new().pipe(TrackingMiddleware { name: "base" });

    let branch_a = base_pipeline
        .clone()
        .pipe(TrackingMiddleware { name: "branch_a" });
    let branch_b = base_pipeline
        .clone()
        .pipe(TrackingMiddleware { name: "branch_b" });

    assert_eq!(base_pipeline.len(), 1);
    assert_eq!(branch_a.len(), 2);
    assert_eq!(branch_b.len(), 2);

    let mut ctx_a = TestContext::default();
    let res_a = block_on(branch_a.run(
        &mut ctx_a,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("res_a".to_string()) })
        }),
    ));
    assert_eq!(res_a, Ok("res_a".to_string()));
    assert_eq!(
        ctx_a.log,
        vec!["base:start", "branch_a:start", "branch_a:end", "base:end"]
    );

    let mut ctx_b = TestContext::default();
    let res_b = block_on(branch_b.run(
        &mut ctx_b,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("res_b".to_string()) })
        }),
    ));
    assert_eq!(res_b, Ok("res_b".to_string()));
    assert_eq!(
        ctx_b.log,
        vec!["base:start", "branch_b:start", "branch_b:end", "base:end"]
    );

    let merged = branch_a.merge(branch_b);
    assert_eq!(merged.len(), 4);
}

#[test]
fn test_run_through_with_unit_output() {
    let pipeline = Pipeline::new().pipe_fn(
        |ctx: &mut TestContext, next: Next<TestContext, (), AppError>| {
            Box::pin(async move {
                ctx.counter += 10;
                next.run(ctx).await?;
                ctx.counter += 5;
                Ok(())
            })
        },
    );

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run_through(&mut ctx));
    assert_eq!(result, Ok(()));
    assert_eq!(ctx.counter, 15);
}

#[test]
fn test_empty_pipeline_calls_terminal_directly() {
    let pipeline: Pipeline<TestContext, String, AppError> = Pipeline::default();
    assert!(pipeline.is_empty());
    assert_eq!(pipeline.len(), 0);

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            Box::pin(async move {
                target_ctx.counter = 99;
                Ok("direct".to_string())
            })
        }),
    ));

    assert_eq!(result, Ok("direct".to_string()));
    assert_eq!(ctx.counter, 99);
}

#[test]
fn test_pipeline_error_helpers_and_display() {
    let err_double: PipelineError<AppError> = PipelineError::NextAlreadyCalled;
    assert!(err_double.is_next_already_called());
    assert_eq!(err_double.execution(), None);
    assert_eq!(err_double.clone().into_execution(), None);
    assert_eq!(
        format!("{err_double}"),
        "pipeline step next.run() was invoked more than once"
    );

    let err_exec: PipelineError<AppError> =
        PipelineError::Execution(AppError::NotFound("item".to_string()));
    assert!(!err_exec.is_next_already_called());
    assert_eq!(
        err_exec.execution(),
        Some(&AppError::NotFound("item".to_string()))
    );
    assert_eq!(format!("{err_exec}"), "not found: item");

    let transformed = err_exec.map_execution(|err| format!("mapped:{err}"));
    assert_eq!(
        transformed,
        PipelineError::Execution("mapped:not found: item".to_string())
    );

    let std_err: &dyn std::error::Error = &PipelineError::Execution(AppError::Unauthorized);
    assert!(std_err.source().is_some());
    let double_std: &dyn std::error::Error = &PipelineError::<AppError>::NextAlreadyCalled;
    assert!(double_std.source().is_none());
}

#[test]
fn test_concurrent_multi_threaded_execution() {
    let pipeline = Arc::new(
        Pipeline::new()
            .pipe(TrackingMiddleware { name: "m1" })
            .pipe(TrackingMiddleware { name: "m2" }),
    );

    let success_count = Arc::new(AtomicUsize::new(0));
    let mut thread_handles = Vec::new();

    for index in 0..10 {
        let pipeline_clone = Arc::clone(&pipeline);
        let success_clone = Arc::clone(&success_count);

        let handle = std::thread::spawn(move || {
            let mut ctx = TestContext::default();
            let result = block_on(pipeline_clone.run(
                &mut ctx,
                from_terminal(move |target_ctx: &mut TestContext| {
                    let _ = target_ctx;
                    Box::pin(async move { Ok(format!("resp_{index}")) })
                }),
            ));

            if result == Ok(format!("resp_{index}"))
                && ctx.log == vec!["m1:start", "m2:start", "m2:end", "m1:end"]
            {
                success_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        thread_handles.push(handle);
    }

    for handle in thread_handles {
        handle.join().expect("thread panicked");
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 10);
}

#[test]
fn test_from_fn_helper_constructor() {
    let middleware = from_fn(
        |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
            Box::pin(async move {
                ctx.counter += 42;
                next.run(ctx).await
            })
        },
    );
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("from_fn_ok".to_string()) })
        }),
    ));
    assert_eq!(result, Ok("from_fn_ok".to_string()));
    assert_eq!(ctx.counter, 42);
}

#[test]
fn test_next_methods_and_debug_format() {
    let pipeline = Pipeline::new().pipe_fn(
        |ctx: &mut TestContext, next: Next<TestContext, String, AppError>| {
            Box::pin(async move {
                assert_eq!(next.current_index(), 1);
                assert!(!next.is_called());
                let debug_repr = format!("{next:?}");
                assert!(debug_repr.contains("Next"));
                assert!(debug_repr.contains("index: 1"));

                let res = next.run(ctx).await;
                assert!(next.is_called());
                res
            })
        },
    );

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("ok".to_string()) })
        }),
    ));
    assert_eq!(result, Ok("ok".to_string()));

    let pipeline_debug = format!("{pipeline:?}");
    assert!(pipeline_debug.contains("Pipeline"));
    assert!(pipeline_debug.contains("middleware_count: 1"));
}

#[test]
fn test_error_handler_invoked_exactly_once_when_not_recovered() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&call_count);

    let pipeline = Pipeline::new()
        .pipe(TrackingMiddleware { name: "layer1" })
        .pipe(TrackingMiddleware { name: "layer2" })
        .pipe(TrackingMiddleware { name: "layer3" });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run_with_error_handler(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Err(AppError::Internal("fatal error".to_string())) })
        }),
        from_error_handler(
            move |error_ctx: &mut TestContext, error: PipelineError<AppError>| {
                let counter = Arc::clone(&count_clone);
                Box::pin(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    error_ctx.log.push("handler_invoked".to_string());
                    // Re-throw without recovering
                    Err(error)
                })
            },
        ),
    ));

    assert_eq!(
        result,
        Err(PipelineError::Execution(AppError::Internal(
            "fatal error".to_string()
        )))
    );
    // Crucial assertion: error handler must be invoked exactly once, not 3 or 4 times!
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        ctx.log,
        vec![
            "layer1:start",
            "layer2:start",
            "layer3:start",
            "handler_invoked"
        ]
    );
}

#[test]
fn test_pipeline_push_and_pipe_arc() {
    let mut pipeline = Pipeline::new();
    pipeline.push(TrackingMiddleware { name: "m1" });

    let boxed_m2: Arc<dyn Middleware<TestContext, String, AppError>> =
        Arc::new(TrackingMiddleware { name: "m2" });
    pipeline.push_arc(boxed_m2);

    let boxed_m3: Arc<dyn Middleware<TestContext, String, AppError>> =
        Arc::new(TrackingMiddleware { name: "m3" });
    let pipeline = pipeline.pipe_arc(boxed_m3);

    assert_eq!(pipeline.len(), 3);

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("finished".to_string()) })
        }),
    ));

    assert_eq!(result, Ok("finished".to_string()));
    assert_eq!(
        ctx.log,
        vec![
            "m1:start", "m2:start", "m3:start", "m3:end", "m2:end", "m1:end",
        ]
    );
}

#[test]
fn test_run_through_with_error_handler() {
    let pipeline: Pipeline<TestContext, (), AppError> =
        Pipeline::new().pipe_fn(|ctx: &mut TestContext, next| {
            Box::pin(async move {
                ctx.counter += 10;
                let res = next.run(ctx).await;
                ctx.counter += 5;
                res
            })
        });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run_through_with_error_handler(
        &mut ctx,
        from_error_handler(|target_ctx: &mut TestContext, err| {
            Box::pin(async move {
                target_ctx.counter += 100;
                Err(err)
            })
        }),
    ));

    assert_eq!(result, Ok(()));
    assert_eq!(ctx.counter, 15);
}

#[test]
fn test_pipeline_error_extended_methods() {
    let mut exec_err: PipelineError<AppError> = PipelineError::from(AppError::Unauthorized);
    assert!(exec_err.is_execution());
    assert!(!exec_err.is_next_already_called());

    if let Some(err_ref) = exec_err.execution_mut() {
        *err_ref = AppError::NotFound("altered".to_string());
    }

    assert_eq!(
        exec_err.clone().into_execution(),
        Some(AppError::NotFound("altered".to_string()))
    );

    let mapped = exec_err.map_execution(|err| format!("{err}"));
    assert_eq!(
        mapped,
        PipelineError::Execution("not found: altered".to_string())
    );

    let next_err: PipelineError<AppError> = PipelineError::NextAlreadyCalled;
    let mapped_next = next_err.map_execution(|err| format!("{err}"));
    assert_eq!(mapped_next, PipelineError::NextAlreadyCalled);
}

#[test]
fn test_next_remaining_and_is_terminal() {
    let pipeline: Pipeline<TestContext, String, AppError> = Pipeline::new()
        .pipe_fn(|ctx, next| {
            Box::pin(async move {
                assert_eq!(next.remaining(), 1);
                assert!(!next.is_terminal());
                next.run(ctx).await
            })
        })
        .pipe_fn(|ctx, next| {
            Box::pin(async move {
                assert_eq!(next.remaining(), 0);
                assert!(next.is_terminal());
                next.run(ctx).await
            })
        });

    let mut ctx = TestContext::default();
    let result = block_on(pipeline.run(
        &mut ctx,
        from_terminal(|target_ctx: &mut TestContext| {
            let _ = target_ctx;
            Box::pin(async move { Ok("ok".to_string()) })
        }),
    ));

    assert_eq!(result, Ok("ok".to_string()));
}
