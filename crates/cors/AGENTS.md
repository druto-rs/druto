# `cors` Agent Instructions

## Scope

This crate owns Cross-Origin Resource Sharing (CORS) configuration, preflight handling, and header injection.

## Design Rules

1. **W3C / RFC 6454 Compliance**:
    - Follow W3C recommendations strictly. Never return `Access-Control-Allow-Origin: *` when
      `Access-Control-Allow-Credentials: true`.
2. **Short-Circuit Preflight Requests**:
    - Preflight `OPTIONS` requests must terminate within the CORS middleware without invoking downstream pipeline layers
      or database handlers.
3. **Pipeline & Router Integration**:
    - Implement `pipeline::Middleware<HttpContext, Response, RouterError>`.
4. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.
