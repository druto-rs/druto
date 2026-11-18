# `pipeline` Crate Context

## Purpose

`pipeline` provides a composable, generic asynchronous chain-of-responsibility (onion) execution pipeline for the Druto framework. It allows an arbitrary context and request/response lifecycle to be passed through a sequential series of interceptors (middleware) wrapping a terminal handler.

## Reference Architecture

This crate is the Rust equivalent of:
- **AdonisJS**: `@poppinss/middleware` (`Runner`, `once`)
- **Laravel**: `illuminate/pipeline` (`Pipeline::send()->through()->then()`)
- **Node.js/Koa**: Onion middleware execution model (`await next()`)

In Rust, implementing an async onion pipeline is challenging due to recursive futures, lifetimes, and object-safety constraints across thread boundaries. `pipeline` solves this problem once as a clean, reusable primitive.

## Core Responsibilities

1. **Pipeline Composition**:
   - Assemble an ordered list of middleware stages.
   - Maintain a terminal handler executed at the center of the onion.
2. **Execution Flow (Onion Model)**:
   - Middleware executes pre-processing logic.
   - Middleware calls `next.run(context).await` to yield control downstream.
   - Downstream middleware and handler execute.
   - Control returns to middleware for post-processing logic (inspecting or altering response, timing, headers).
3. **Short-Circuiting & Flow Control**:
   - Middleware can return early without calling `next`, cleanly halting downstream execution.
   - Only allow `next.run()` to be called once per middleware invocation; subsequent calls must error or no-op safely.
4. **Error Propagation & Interception**:
   - Errors produced downstream bubble up through upstream middleware layers, allowing upstream middleware to catch, log, or transform errors.

## Boundaries & Non-Responsibilities

- **HTTP Agnostic**: Does not know what an HTTP request or response is. The context `C` and result/response `R` are generic type parameters.
- **Service Independence**: Independent of Tower, Hyper, or Axum.
- **Reusable Across Subsystems**: Can be used for HTTP middleware, database query hooks, command pipelines, or background job processing.

## Target Public Surface

- `trait Middleware<Context, Output>`: The asynchronous middleware contract.
- `struct Pipeline<Context, Output>`: The builder and runner for the pipeline.
- `struct Next<'a, Context, Output>`: The handle passed to each middleware to advance execution.
- `PipelineError`: Pipeline-specific errors (e.g. calling `next` multiple times).
