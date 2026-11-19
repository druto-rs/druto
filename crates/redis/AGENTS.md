# `redis` Agent Instructions

## Scope

This crate owns Redis client connection pooling, command execution pipelines, pub/sub channels, transactions, and
distributed driver integrations.

## Design Rules

1. **Multiplexing & Connection Safety**:
    - Manage multiplexed async connections with graceful reconnection handling.
    - Pub/Sub connections must be isolated from standard command/query connection pools.

2. **Atomic Operations**:
    - Atomic mutations across multiple keys must utilize atomic pipelines, transactions, or Lua scripts.

3. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

4. **Framework-Agnostic Documentation & Codebase**:
    - Do not mention other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
    - Reference only Redis specifications and Druto types.
