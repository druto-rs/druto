# `limiter` Agent Instructions

## Scope

This crate owns rate limiting, burst throttling, token bucket and sliding window algorithms, storage backends, and
HTTP rate limit headers.

## Design Rules

1. **Precision & Thread Safety**:
    - Rate limit state updates must be atomic and race-condition free under high concurrency.
    - Distributed drivers (e.g. Redis) must execute atomic operations (single roundtrip or atomic Lua scripts).

2. **HTTP Standards Compliance**:
    - Emits accurate `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and `Retry-After` headers.
    - Default response status on exhaustion is `429 Too Many Requests`.

3. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

4. **Framework-Agnostic Documentation & Codebase**:
    - Do not mention other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
    - Reference only RFCs, web standards, and Druto types.
