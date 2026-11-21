# `cors` Crate Context & Architecture

## Vision & Scope

The `cors` crate provides standard Cross-Origin Resource Sharing (CORS) enforcement, preflight negotiation, and header
management for the Druto web framework. It operates as an asynchronous onion middleware layer on top of `dhap` and
`router`, handling both preflight `OPTIONS` requests and actual cross-origin HTTP requests in accordance with the
[W3C CORS Specification](https://www.w3.org/TR/cors/) and [RFC 6454](https://datatracker.ietf.org/doc/html/rfc6454).

## Reference Blueprint & Sibling Mapping

The crate synthesizes architectural patterns from industry-standard frameworks:

- **AdonisJS (`@adonisjs/cors`)**:
    - Request short-circuiting on preflight `OPTIONS` with `204 No Content` and empty body.
    - Normalized exposed headers, filtering out standard simple response headers (`cache-control`, `content-language`,
      `content-type`, `expires`, `last-modified`, `pragma`).
    - Dynamic request inspection and origin reflection.
    - Default header reflection (`headers: true`) and default preflight cache (`maxAge: 90`).
- **Laravel (`fruitcake/laravel-cors` / `Illuminate\Http\Middleware\HandleCors`)**:
    - Path-scoped activation (`paths: Vec<String>`, e.g. `["api/*"]`), ensuring CORS overhead and headers only apply to
      explicitly designated API route hierarchies.
    - Subdomain wildcard patterns (`*.example.com`).
    - Strict `Vary` header management (`Vary: Origin`, `Vary: Access-Control-Request-Method`,
      `Vary: Access-Control-Request-Headers`) to prevent intermediate CDN and reverse proxy cache poisoning.
- **Tower HTTP (`tower-http::cors::CorsLayer`) & Loco.rs (`loco_rs::controller::middleware::cors`)**:
    - Strongly typed origin policies (`Any`, `Exact`, `Pattern`, `Dynamic`).
    - Static and runtime validation for W3C compatibility.

## Key Capabilities & Core Responsibilities

1. **Preflight Interception & Fail-Closed Short-Circuiting**:
    - Identifies preflight requests: HTTP `OPTIONS` requests carrying an `Origin` header and an
      `Access-Control-Request-Method` header.
    - Validates the requested method against the allowed method policy.
    - Validates requested headers (`Access-Control-Request-Headers`) against the allowed headers policy.
    - **When valid**: responds immediately with status `204 No Content` (configurable to `200 OK` for legacy client
      support), sets configured `Access-Control-Allow-*` headers, and terminates execution without calling downstream
      middleware or route handlers.
    - **When invalid**: responds immediately with `204 No Content` omitting all `Access-Control-Allow-*` headers per
      the W3C specification, failing closed and protecting downstream handlers and database layers from execution.

2. **Flexible Origin Negotiation & Credential Invariant**:
    - `OriginPolicy::Any`: Wildcard `*` for public APIs. When paired with `allow_credentials: true`, automatically
      reflects the caller's `Origin` header (along with `Vary: Origin`) to maintain full browser compatibility without
      developer friction.
    - `OriginPolicy::Exact(Vec<String>)`: Strict case-sensitive match against an explicit whitelist of origins (e.g.
      `https://app.example.com`).
    - `OriginPolicy::Pattern(Vec<String>)`: Wildcard subdomain matching (e.g. `https://*.example.com`).
    - `OriginPolicy::Dynamic`: Closure or trait predicate `Fn(&str, &HttpContext) -> bool` for runtime origin
      resolution (e.g. database tenant domain lookup).
    - `OriginPolicy::Mirror`: Explicit reflection of the incoming request origin.

3. **Method & Header Enforcement**:
    - `Access-Control-Allow-Methods`: Configurable list of permitted HTTP verbs (`GET`, `POST`, `PUT`, `PATCH`,
      `DELETE`, `HEAD`, `OPTIONS`). Default permits all standard methods.
    - `Access-Control-Allow-Headers`: Configurable whitelist of permitted headers or dynamic reflection of
      `Access-Control-Request-Headers`. Defaults to `HeaderPolicy::Mirror` for seamless SPA developer ergonomics.
    - `Access-Control-Expose-Headers`: Whitelist of headers exposed to client-side scripts, with automatic normalization
      excluding default simple response headers (`cache-control`, `content-language`, `content-type`, `expires`,
      `last-modified`, `pragma`).
    - `Access-Control-Max-Age`: Caching duration (in seconds) for preflight responses in browser caches. Defaults to
      `Some(90)` seconds.
    - `Access-Control-Allow-Credentials`: Boolean flag indicating whether cookies, authorization headers, or TLS client
      certificates are allowed.

4. **Path-Scoped Execution (`paths`)**:
    - Configurable URI path prefixes (e.g. `paths: ["api/*", "webhooks/*"]`). Defaults to `["*"]` (all paths).
      Requests outside the configured paths bypass CORS processing with zero overhead.

5. **Cache Poisoning Defense (`Vary` Headers)**:
    - Automatically appends `Vary: Origin` whenever origin matching is non-wildcard or reflected.
    - Appends `Vary: Access-Control-Request-Method` and `Vary: Access-Control-Request-Headers` on preflight responses to
      ensure intermediate caches (CDNs, proxies) do not serve preflight responses across disparate clients.

6. **Pipeline Middleware Integration**:
    - Implements `dhap::Middleware<HttpContext, Response, RouterError>`, allowing seamless attachment as global
      middleware or route-group middleware within the Druto pipeline.

## Boundaries & Non-Responsibilities

- **Not an HTTP Server**: Does not bind sockets or manage TCP/TLS listeners; operates exclusively on `router::Request`
  and `router::Response`.
- **Not CSRF / CSP Defense**: Does not generate CSRF tokens or enforce Content Security Policy; those responsibilities
  belong strictly to `crates/shield`.
- **Not Authentication**: Does not validate bearer tokens, session identifiers, or user credentials; only conveys
  browser permissions via `Access-Control-Allow-Credentials`.

## Public Surface

- `struct CorsConfig`: Comprehensive configuration options for CORS behavior.
- `struct CorsConfigBuilder`: Fluent builder for constructing `CorsConfig`.
- `enum OriginPolicy`: Strongly typed origin matching strategies (`Any`, `Exact`, `Pattern`, `Mirror`, `Dynamic`).
- `enum HeaderPolicy`: Allowed headers strategies (`Mirror`, `List(Vec<HeaderName>)`).
- `enum MethodPolicy`: Allowed HTTP methods strategies (`Any`, `List(Vec<Method>)`).
- `struct CorsMiddleware`: Pipeline middleware implementing `dhap::Middleware<HttpContext, Response, RouterError>`.
