# `events` Agent Instructions

## Scope

This crate owns asynchronous in-process event dispatching, typed listener registration, wildcard topics, and testing
doubles.
It must remain decoupled from specific database or HTTP types.

## Design Rules

1. **Async & Thread Safety**:
    - All event types, listener closures, and futures must satisfy `Send + Sync + 'static` to allow safe dispatch across
      multi-threaded executors (e.g. Tokio).
2. **Deterministic Execution Order**:
    - For a given event, registered listeners must be invoked strictly in registration order unless explicitly
      configured for concurrent execution.
3. **Test Double Isolation**:
    - When `fake()` is enabled, events must be recorded in memory and prevented from invoking live handlers that produce
      external side effects.
4. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`).
5. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
6. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **Type-Safe Dispatch**: Test registering multiple typed listeners and asserting all receive the emitted event.
- **Order Invariant**: Test that listeners execute sequentially in registration order.
- **Wildcard Topic Matching**: Test that pattern listeners (e.g. `"order.*"`) trigger for `"order.created"` and
  `"order.cancelled"`.
- **Error Policy**: Test error propagation when a listener fails under fail-fast vs ignore error policies.
- **Fake Assertions**: Test `EventFake` methods (`assert_dispatched`, `assert_not_dispatched`, `count`) across multiple
  event types.
