# `session` Crate Context

## Purpose

`session` provides multi-driver HTTP session management, flash message storage, and session fixation defense for the
Druto framework. It allows stateful user interactions across HTTP requests by bridging `cookie`, `crypto`, and
`pipeline`.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/session` (`Session`, `SessionManager`, drivers: `cookie`, `memory`, `file`, `redis`)
- **Laravel**: `illuminate/session` (`SessionManager`, `Store`, `StartSession` middleware)
- **Topcoat**: `topcoat-session`

## Core Responsibilities

1. **Multi-Driver Storage Architecture**:
    - Generic storage contract (`trait SessionStore: Send + Sync + 'static`):
        - **Cookie Driver**: Stores the entire encrypted payload directly in a tamper-proof cookie via `crates/cookie`.
          Zero server-side state.
        - **Memory Driver**: Fast in-memory storage ideal for automated test suites.
        - **File Driver**: Local disk-backed session persistence.
2. **Session Fixation Defense & ID Management**:
    - Session identifiers generated using cryptographically secure random bytes via system CSPRNG.
    - `regenerate()`: Generates a fresh session ID while retaining existing session data. Must be called upon user
      authentication
      to prevent session fixation attacks.
3. **Flash Data Storage**:
    - Transient key-value data preserved only until the subsequent HTTP request.
    - Methods: `flash(key, value)`, `reflash()` (persists all current flash data for another request), `keep(keys)`, and
      `old(key)`.
4. **Middleware Lifecycle Integration**:
    - Integrates cleanly with `crates/pipeline`:
        - Reads the session identifier or cookie at the start of the request lifecycle.
        - Exposes session accessors to downstream handlers.
        - Commits changes to the storage driver and appends the session cookie to the response during upstream
          unwinding.

## Boundaries & Non-Responsibilities

- **Not Authentication**: Manages generic session key-value state. User models, password checking, and auth guards
  belong in `crates/auth`.
- **Not an HTTP Server**: Integrates via request/response cookies and middleware abstractions without owning sockets.

## Proposed Public Surface

- `struct Session`: Active request session providing `get`, `put`, `remove`, `flash`, `old`, and `regenerate`.
- `trait SessionStore`: Storage driver interface (`read`, `write`, `destroy`, `touch`).
- `struct SessionManager`: Configuration facade and driver registry.
- `struct SessionConfig`: Session cookie name, lifetime, secure/http-only flags, and driver choice.
- `enum SessionError`: Strongly typed errors for storage failures, serialization issues, and expired sessions.
