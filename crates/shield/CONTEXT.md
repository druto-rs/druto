# `shield` Crate Context & Architecture

## Vision & Scope

The `shield` crate provides comprehensive HTTP web security hardening and request guard middlewares for the Druto
framework. It bundles defenses against common web application vulnerabilities (CSRF, Clickjacking, MIME confusion,
XSS, and protocol downgrades).

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/shield` (CSRF guard, CSP, FrameGuard, HSTS, NoSniff)
- **Laravel**: `Illuminate\Foundation\Http\Middleware\ValidateCsrfToken` (formerly `VerifyCsrfToken`)
- **OWASP**: OWASP Secure Headers Project

## Key Capabilities & Responsibilities

1. **CSRF Protection (Cross-Site Request Forgery)**:
    - Token generation, rotation, and cryptographic verification for mutating HTTP methods (`POST`, `PUT`, `PATCH`,
      `DELETE`).
    - Supports both session-backed token storage and double-submit encrypted cookie patterns.
    - Extracts incoming tokens from headers (`X-CSRF-TOKEN`, `X-XSRF-TOKEN`) or form bodies (`_csrf`).
    - Route path exclusion whitelisting (e.g. for incoming webhooks or third-party callback routes).

2. **Content Security Policy (CSP)**:
    - Declarative policy builder supporting standard directives (`default-src`, `script-src`, `style-src`, `img-src`,
      `connect-src`, `font-src`, `object-src`).
    - Per-request cryptographic nonce generation injected into `HttpContext` extensions for inline script/style tags.
    - Report-only mode (`Content-Security-Policy-Report-Only`) support.

3. **FrameGuard (Clickjacking Defense)**:
    - Configures `X-Frame-Options` header (`DENY`, `SAMEORIGIN`, or disabled when framing is permitted).

4. **NoSniff (MIME-Confusion Defense)**:
    - Injects `X-Content-Type-Options: nosniff` to prevent browsers from interpreting non-executable types as executable
      scripts.

5. **HSTS (HTTP Strict Transport Security)**:
    - Emits `Strict-Transport-Security` headers with configurable `max-age`, `includeSubDomains`, and `preload` flags
      for HTTPS traffic.

6. **Referrer & Permissions Policies**:
    - `Referrer-Policy` (e.g. `no-referrer-when-downgrade`, `strict-origin-when-cross-origin`).
    - `Permissions-Policy` controlling browser feature access (geolocation, camera, microphone, etc.).
