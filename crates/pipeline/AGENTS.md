# `pipeline` Agent Instructions

## Scope

This crate owns the generic asynchronous chain-of-responsibility runner. It must remain decoupled from HTTP, networking, or specific application contexts.

## Design Rules

1. **Async & Thread Safety**:
   - All middleware types and execution futures must satisfy `Send + Sync + 'static` to be safely used across multi-threaded async executors (e.g. Tokio worker pools).
2. **Onion Order Invariant**:
   - Upstream before-phase runs in registration order ($1 \to 2 \to \dots \to N$).
   - Downstream after-phase runs in reverse registration order ($N \to \dots \to 2 \to 1$).
3. **Double `next` Protection**:
   - Calling `next.run()` more than once in the same middleware must fail predictably (e.g., returning an error) rather than running downstream handlers twice.

## Testing Requirements

- **Execution Order Verification**: Test that order of execution strictly preserves the onion model for multiple middleware layers.
- **Short-Circuit Tests**: Verify that returning early without calling `next` prevents subsequent middleware and the terminal handler from running.
- **Error Propagation**: Test that errors returned from terminal handlers or downstream middleware are catchable and transformable by upstream middleware.
- **Multiple `next` Invocations**: Explicit test ensuring that attempting to call `next` twice is prevented.
