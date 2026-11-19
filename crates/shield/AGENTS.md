# `shield` Agent Instructions

## Scope

This crate owns HTTP application security hardening, CSRF token generation and validation, Content Security Policy (CSP)
with per-request nonces, and security headers enforcement.

## Design Rules

1. **Defense-in-Depth & Safe Defaults**:
    - Security headers must default to modern secure stances per OWASP guidelines (e.g.
      `X-Content-Type-Options: nosniff`,
      `X-Frame-Options: SAMEORIGIN`, `Referrer-Policy: strict-origin-when-cross-origin`).
    - Safe methods (`GET`, `HEAD`, `OPTIONS`, `TRACE`) must be exempt from CSRF validation. Mutating methods (`POST`,
      `PUT`, `PATCH`, `DELETE`) require token validation.

2. **Timing-Attack Resistance**:
    - CSRF token comparison against session secrets must use constant-time equality comparisons
      (`subtle::ConstantTimeEq`) to eliminate timing side-channels.

3. **Session & Cookie Integration**:
    - CSRF secret storage must integrate with `session::SessionContext` via `router::HttpContext`.
    - If session middleware is active, store CSRF secret in the session.
    - If `enable_xsrf_cookie` is true, stage the `XSRF-TOKEN` cookie in the request `CookieJar`.

4. **Non-Destructive Header Manipulation**:
    - When applying security headers, do not clobber headers already explicitly set by downstream route handlers or
      upstream middleware.

