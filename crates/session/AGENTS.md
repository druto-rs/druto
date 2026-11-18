# `session` Agent Instructions

## Scope

This crate owns HTTP session state management, multi-driver storage backends, flash data, and session fixation defense.
It bridges `cookie`, `crypto`, and `pipeline` without binding to concrete HTTP server implementations.

## Design Rules

1. **Session Fixation Defense Invariant**:
    - `regenerate()` must issue a completely new random session ID while migrating existing data.
2. **Flash Lifecycle Invariant**:
    - Data stored via `flash()` is only accessible on the immediate subsequent request and must be purged on the
      following request unless `reflash()` or `keep()` is invoked.
3. **Async & Thread Safety**:
    - `SessionStore` implementations must be `Send + Sync + 'static` to allow concurrent access across worker threads.
4. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`).
5. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
6. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **Driver Roundtrips**: Test store, retrieve, update, and destroy across `MemorySessionStore`, `CookieSessionStore`,
  and `FileSessionStore`.
- **Flash Data Lifecycle**: Verify that flash data exists on request $N+1$, and is automatically discarded by
  request $N+2$.
- **Reflash & Keep**: Verify `reflash` and `keep` preserve flash data across multiple requests.
- **Session Regeneration**: Test that `regenerate()` changes the session ID while retaining all existing data.
