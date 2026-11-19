# `shield` Agent Instructions

## Scope

This crate owns HTTP application security guards, CSRF token validation, Content Security Policy (CSP), and security
headers enforcement.

## Design Rules

1. **Defense-in-Depth & Safe Defaults**:
    - Security headers must default to modern secure stances (e.g. `X-Content-Type-Options: nosniff`,
      `X-Frame-Options: SAMEORIGIN`).
2. **Timing-Attack Resistance**:
    - CSRF token verification must use constant-time equality comparisons (`subtle::ConstantTimeEq`).
3. **Session & Cookie Integration**:
    - Integrate seamlessly with `session::SessionContext` and `cookie::CookieJar` via `router::HttpContext`.
4. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.
