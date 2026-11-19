# `cors` Agent Instructions

## Scope

This crate owns Cross-Origin Resource Sharing (CORS) configuration, preflight request negotiation, header injection,
and intermediate cache defense (`Vary` header management).

## Design Rules

1. **W3C / RFC 6454 Compliance**:
    - Strictly adhere to the W3C CORS specification and RFC 6454.
    - Never emit `Access-Control-Allow-Origin: *` when `Access-Control-Allow-Credentials: true`. If credentials are
      enabled, the allowed origin must be an explicit origin or reflected origin.
    - Case-sensitive origin matching: Origin comparisons must be case-sensitive per RFC 6454.
    - Case-insensitive header matching: Header names in `Access-Control-Request-Headers` must be matched
      case-insensitively.

2. **Short-Circuit Preflight Requests**:
    - Preflight `OPTIONS` requests carrying `Origin` and `Access-Control-Request-Method` must terminate within the CORS
      middleware without invoking downstream pipeline layers or database handlers.
    - Return `204 No Content` with an empty response body.

3. **Cache Invalidation & Vary Hygiene**:
    - When origin negotiation depends on request headers (i.e. not an unconditional `*`), always include `Vary: Origin`
      on cross-origin responses.
    - Preflight responses must include `Vary: Access-Control-Request-Method` and
      `Vary: Access-Control-Request-Headers`.

4. **Pipeline & Router Integration**:
    - Implement `pipeline::Middleware<HttpContext, Response, RouterError>`.
    - Non-destructive header manipulation: augment existing headers without wiping headers set by upstream layers.

5. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

6. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
    - Document public surfaces with rustdoc (`///`, `//!`).
