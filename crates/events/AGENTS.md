# `events` Agent Instructions

## Scope

This crate owns asynchronous in-process event dispatching, typed listener registration, wildcard topic routing, and
testing doubles. It must remain decoupled from specific database, authentication, or HTTP types, providing a generic
foundation for the framework.

## Design Rules

1. **Async & Thread Safety**:
    - All event types, listener closures, and returned futures must satisfy `Send + Sync + 'static` to ensure seamless
      and safe dispatch across multi-threaded asynchronous runtimes (e.g., Tokio, smol).
2. **Deterministic Execution Order & Concurrency**:
    - For a given event, registered listeners must be invoked strictly in registration order by default (`emit` /
      `emit_serial`). Concurrent execution (`emit_concurrent`) must be an explicit opt-in.
3. **Re-entrancy Safety & Snapshot-on-Dispatch**:
    - Internal locks must never be held across `.await` points. The dispatcher must snapshot listener pointers under a
      brief read lock and release the lock before awaiting listener futures, preventing deadlocks during nested
      emissions.
4. **Short-Circuiting & Halting Semantics**:
    - When executing under `emit_until`, execution halts immediately upon encountering the first listener returning
      `ControlFlow::Break(val)`. Remaining listeners must not be invoked.
5. **Wildcard Topic Matching**:
    - Topic matching must use dot-delimited segment matching: `*` matches a single segment; `**` matches multi-segment
      descendants.
6. **Test Double Isolation & Selective Faking**:
    - When faking is enabled (`fake()` or `fake_only::<E>()`), emitted events must be buffered in memory and matched
      handlers prevented from invoking external side effects.
    - Selective faking must only buffer the specified event types, allowing un-faked event types to dispatch to live
      listeners normally.
7. **Zero Runtime Lock-in**:
    - The core crate must not depend directly on an opinionated async runtime (such as `tokio`) in `[dependencies]`.
      Use standard `std::future::Future` and `std::pin::Pin` for futures. Runtime-specific helpers or Tokio join sets
      should be test-only or feature-gated.
8. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code to resolve
      warnings cleanly.
9. **No Leading Underscores**:
    - Do not use leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
10. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces thoroughly with rustdoc (`///`, `//!`).

## Testing Requirements

- **Type-Safe Dispatch**: Verify registering multiple typed listeners and asserting all receive the emitted event.
- **Order Invariant**: Test that listeners execute sequentially in strict registration order under `emit_serial`.
- **Concurrent Dispatch**: Test that `emit_concurrent` triggers all registered listeners concurrently.
- **Short-Circuiting (`emit_until`)**: Test that returning `ControlFlow::Break` halts execution immediately, leaving
  subsequent listeners uncalled.
- **Wildcard Topic Matching**: Test that pattern listeners (e.g. `"order.*"`, `"user.**"`, `"*"`) match expected topics
  and reject non-matching paths.
- **Nested Emission & Re-entrancy**: Test that a listener emitting a secondary event executes safely without deadlocks.
- **Error Strategy**: Verify error propagation under `ErrorStrategy::FailFast` (immediate abort) vs
  `ErrorStrategy::Collect` (accumulate all errors), and verify `on_error` hook triggering.
- **One-Time Listener (`once`)**: Test that a `once` listener fires exactly once and is automatically removed for
  subsequent emissions.
- **Deregistration (`off`)**: Test that unsubscribing a listener ID prevents it from firing.
- **Conditional Registration (`listen_if`)**: Test that false conditions prevent listener registration.
- **Fake Assertions**: Test `EventFake` assertion methods (`assert_dispatched`, `assert_dispatched_times`,
  `assert_not_dispatched`, `assert_none_dispatched`, and payload inspections) across multiple event types.
- **Selective Faking**: Test that faking only event `A` leaves event `B` dispatching to real listeners.
