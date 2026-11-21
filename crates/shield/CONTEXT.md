# `shield` Crate Context & Architecture

## Vision & Scope

The `shield` crate provides comprehensive HTTP web security hardening, request guard middlewares, and security header
enforcement for the Druto framework. It bundles defenses against common web application vulnerabilities (Cross-Site
Request Forgery, Clickjacking, MIME confusion, Cross-Site Scripting, and protocol downgrades) compliant with OWASP
guidelines.

## Architectural Patterns & Specifications

The crate synthesizes security patterns conforming to modern web standards:

- **Industry Security Standards**:
    - OWASP Top 10 Web Application Security Risks and OWASP Secure Headers Project.
    - W3C Content Security Policy Level 3 (`Content-Security-Policy`, `@nonce` per-request CSPRNG nonces).
    - RFC 6797 (HTTP Strict Transport Security - HSTS).
    - RFC 7034 (HTTP Header Field X-Frame-Options).
    - W3C Referrer Policy and Permissions Policy specifications.
- **Defense in Depth**:
    - Unified `ShieldMiddleware` coordinating all guards: Security Headers, CSP with per-request nonces, and CSRF
      protection.
    - Multi-source CSRF token extraction (`_csrf` form body/query, `X-CSRF-TOKEN` header, `X-XSRF-TOKEN` header).
    - BREACH-resistant salted/masked CSRF token generation and constant-time verification.
    - Path exclusion whitelisting (glob patterns and custom predicate closures).
    - Automatic `XSRF-TOKEN` cookie emission (`httpOnly: false`) for SPA JavaScript clients.
    - Non-destructive header application (preserving handler-defined response headers).

## Key Capabilities & Core Responsibilities

1. **CSRF Protection (Cross-Site Request Forgery)**:
    - **Secret & Token Lifecycle**: Generates a cryptographically secure random secret stored in the user's session
      (`_csrf_secret`). Emits masked or salted tokens per request (`salt + HMAC(secret, salt)`).
    - **Safe Method Exemption**: Safe HTTP methods (`GET`, `HEAD`, `OPTIONS`, `TRACE`) are exempt from CSRF validation.
    - **Mutating Method Enforcement**: HTTP methods (`POST`, `PUT`, `PATCH`, `DELETE`) require a valid CSRF token.
    - **Multi-Source Token Extraction**: Inspects:
        1. Request headers: `X-CSRF-TOKEN`.
        2. Cookie / AJAX header: `X-XSRF-TOKEN`.
        3. Form payload or query field: `_csrf` or `_token`.
    - **Timing-Attack Resistance**: Compares token against session secret using constant-time equality
      (`subtle::ConstantTimeEq`).
    - **Route Path Exclusions**: Whitelist of path prefixes (e.g. `["/api/webhooks/*", "/stripe/webhook"]`) and custom
      predicate closures `Fn(&HttpContext) -> bool` where CSRF is safely bypassed.
    - **Context Extension**: Stores generated `CsrfToken` in `HttpContext::extensions` for view rendering and form
      helpers.
    - **Optional XSRF Cookie**: Sets readable `XSRF-TOKEN` cookie (`httpOnly: false`) for frontend AJAX / SPA
      integration.

2. **Content Security Policy (CSP)**:
    - Declarative policy builder supporting standard directives (`default-src`, `script-src`, `style-src`, `img-src`,
      `font-src`, `connect-src`, `media-src`, `object-src`, `frame-src`, `worker-src`, `form-action`, `frame-ancestors`,
      `base-uri`).
    - **Per-Request Cryptographic Nonce**: Generates a 128-bit (16-byte) CSPRNG nonce formatted as Base64, injected into
      `HttpContext::extensions` (`CspNonce`) and substituted for `@nonce` placeholders in script and style directives.
    - **Report-Only Mode**: Emits `Content-Security-Policy-Report-Only` when testing new policies without breaking
      client
      execution.

3. **Security Headers & Presets**:
    - **Preset Options**:
        - `Preset::SecureDefaults`: `X-Content-Type-Options: nosniff`, `X-Frame-Options: SAMEORIGIN`,
          `Strict-Transport-Security`, `Referrer-Policy: strict-origin-when-cross-origin`.
        - `Preset::Owasp`: Strict lockdown adding `Cross-Origin-Opener-Policy: same-origin`,
          `Cross-Origin-Embedder-Policy: require-corp`, `Cross-Origin-Resource-Policy: same-origin`, `Clear-Site-Data`,
          and
          `Permissions-Policy`.
        - `Preset::Empty`: Blank canvas for fully custom policies.
    - **FrameGuard (Clickjacking Defense)**: `X-Frame-Options: DENY` or `SAMEORIGIN`.
    - **NoSniff (MIME-Confusion Defense)**: `X-Content-Type-Options: nosniff`.
    - **HSTS (HTTP Strict Transport Security)**: `Strict-Transport-Security: max-age=...; includeSubDomains; preload`.
    - **Referrer & Permissions Policies**: Granular control over referrer leakage and browser feature access.
    - **Non-Destructive Application**: Never overwrites headers already set by downstream route handlers unless
      explicitly configured.

4. **Pipeline Middleware Integration**:
    - Implements `dhap::Middleware<HttpContext, Response, RouterError>`, usable as an all-in-one `ShieldMiddleware`
      or modular individual guard layers (`CsrfMiddleware`, `CspMiddleware`, `SecurityHeadersMiddleware`).

## Boundaries & Non-Responsibilities

- **Not an Authentication System**: Does not authenticate users, verify passwords, or manage login sessions; those
  belong in `crates/auth`.
- **Not CORS Negotiation**: Does not handle cross-origin preflight requests or origin whitelists; those belong strictly
  in `crates/cors`.
- **Transport Agnostic**: Operates over `router::HttpContext` and does not bind sockets or manage TLS certificates.

## Public Surface

- `struct ShieldConfig` & `struct ShieldConfigBuilder`: Unified configuration for all security guards.
- `struct ShieldMiddleware`: Complete middleware applying all active security guards.
- `struct CsrfConfig`: Configuration options for CSRF protection.
- `struct CsrfMiddleware`: Standalone CSRF guard middleware.
- `struct CsrfToken`: Strongly typed CSRF token stored in request extensions.
- `struct CspConfig`: Configuration options for Content Security Policy.
- `struct CspMiddleware`: Standalone CSP guard middleware.
- `struct CspNonce`: Strongly typed cryptographic nonce stored in request extensions.
- `struct SecurityHeadersConfig`: Configuration options for security headers and presets.
- `struct SecurityHeadersMiddleware`: Standalone security headers guard middleware.
- `enum SecurityPreset`: `SecureDefaults`, `Owasp`, `Empty`.
- `enum FrameAction`: `Deny`, `SameOrigin`.
- `enum ReferrerPolicy`: `NoReferrer`, `SameOrigin`, `StrictOriginWhenCrossOrigin`, etc.
- `enum ShieldError`: Typed error representation for CSRF failures, conversion to `RouterError`.

## Verification & Test Coverage

- **Suite**: 39 specification tests + 2 documentation tests covering all guards, presets, token masking, and lifecycle.
- **Coverage**: 96.46% overall line coverage (`cargo llvm-cov`), with 100% line coverage for `headers.rs` and
  `error.rs`, and >93% for all other modules.
- **Timing-Safety**: Constant-time equality checks verified for CSRF verification.
- **Non-Destructive**: Handlers retain full control over pre-existing headers and policies.
