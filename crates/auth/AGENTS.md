# `auth` Agent Instructions

## Scope

This crate owns authentication guards, session and token authenticators, user identity resolution, and authentication
middlewares.

## Design Rules

1. **Defense-in-Depth & Constant-Time Operations**:
    - All credential and token verification checks must execute in constant time (`subtle::ConstantTimeEq`) to eliminate
      timing side-channels.
    - Password hashing and verification must delegate strictly to `crates/hash`.

2. **Session Fixation Prevention**:
    - On login and logout, the active session ID must be rotated / regenerated to prevent fixation attacks.

3. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

4. **Framework-Agnostic Documentation & Codebase**:
    - Do not mention other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
    - Reference only authentication standards and Druto types.
