# `pipeline` Crate Context

## Purpose

`pipeline` provides a composable, generic asynchronous chain-of-responsibility (onion) execution pipeline for the Druto
framework. It allows an arbitrary context and request/response lifecycle to be passed through a sequential series of
interceptors (middleware) wrapping a terminal handler.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@poppinss/middleware` (`Runner`, `once`)
- **Laravel**: `illuminate/pipeline` (`Pipeline::send()->through()->then()`)
- **Node.js/Koa**: Onion middleware execution model (`await next()`)

In Rust, implementing an async onion pipeline is challenging due to recursive futures, lifetimes, and object-safety
constraints across thread boundaries. `pipeline` solves this problem once as a clean, reusable primitive.

## Core Responsibilities

1. **Pipeline Composition**:
    - Assemble an ordered list of middleware stages via fluent `.pipe(...)` and `.pipe_fn(...)`.
    - Immutable and $O(1)$ cheap cloning (`Arc` slices) for thread-safe concurrent sharing and route-group branching.
    - Dynamic terminal handler dispatch supplied at execution time via `pipeline.run(&mut ctx, terminal).await`.
2. **Execution Flow (Onion Model)**:
    - Middleware executes pre-processing logic with direct zero-copy access to `&mut Context`.
    - Middleware calls `next.run(context).await` to yield control downstream.
    - Downstream middleware and handler execute.
    - Control returns to middleware for post-processing logic (inspecting or altering response, timing, headers).
3. **Short-Circuiting & Flow Control**:
    - Middleware can return early without calling `next.run()`, cleanly halting downstream execution.
    - Atomically prevents `next.run()` from being invoked more than once per middleware handle or across clones,
      predictably returning `Err(PipelineError::NextAlreadyCalled)`.
4. **Error Propagation & Recovery**:
    - Downstream application errors are automatically lifted into `PipelineError::Execution(E)`.
    - Errors bubble up through upstream middleware layers, allowing upstream middleware to catch, log, or transform
      errors via standard pattern matching (`match next.run(ctx).await`).
    - Optional `run_with_error_handler` hook enables downstream error recovery into valid `Output` before upstream
      post-processing unwinds.
5. **Runtime Independence**:
    - Zero runtime dependencies (`tokio`, `async-std`, `smol`, etc.) in `[dependencies]`.
    - Built purely on standard library `std::future::Future` and `std::pin::Pin`. Compatible with any async executor or
      single-threaded/Wasm environment.

## Boundaries & Non-Responsibilities

- **HTTP Agnostic**: Does not know what an HTTP request or response is. The context `Context`, output `Output`, and
  error `Error` are generic type parameters.
- **Service Independence**: Independent of Tower, Hyper, or Axum.
- **Reusable Across Subsystems**: Can be used for HTTP middleware, database query hooks, command pipelines, or
  background job processing.

## Public Surface

- `trait Middleware<Context, Output, Error>`: The asynchronous middleware contract.
- `struct Pipeline<Context, Output, Error>`: Thread-safe, fluent pipeline builder and runner.
- `struct Next<Context, Output, Error>`: Handle passed to each middleware to advance execution with double-call
  protection.
- `trait Terminal<Context, Output, Error>`: Contract for the terminal handler executed at the center.
- `trait ErrorHandler<Context, Output, Error>`: Contract for pipeline-level error recovery.
- `enum PipelineError<E>`: Pipeline errors (`NextAlreadyCalled` and `Execution(E)`).
- `type BoxFuture<'a, T>`: Pinned boxed future thread-safe across executors.
- `type BoxedMiddleware<Context, Output, Error>`: Thread-safe Arc-wrapped middleware trait object.
- `type MiddlewareStack<Context, Output, Error>`: Arc-wrapped slice of boxed middleware.
- Functions: `from_fn`, `from_terminal`, `from_error_handler`.
