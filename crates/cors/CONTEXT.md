# `cors` Crate Context & Architecture

## Vision & Scope

The `cors` crate provides standard Cross-Origin Resource Sharing (CORS) enforcement and header negotiation for the
Druto web framework. It operates as an onion middleware layer on top of `druto-pipeline` and `druto-router`, handling
both preflight `OPTIONS` requests and actual cross-origin HTTP requests.

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/cors`
- **Tower HTTP**: `tower-http::cors::CorsLayer`
- **Laravel**: `fruitcake/laravel-cors` / `Illuminate\Http\Middleware\HandleCors`

## Key Capabilities & Responsibilities

1. **Preflight Interception**:
    - Detects preflight requests (`OPTIONS` requests carrying `Origin` and `Access-Control-Request-Method`).
    - Validates the requested method and requested headers against the configured policy.
    - Responds immediately with status `204 No Content` (or configurable `200 OK`) and the appropriate
      `Access-Control-Allow-*` headers, short-circuiting downstream route handlers.

2. **Flexible Origin Negotiation**:
    - `AllowAll`: Wildcard `*` for public APIs (disallowing credentials if `*` is used per W3C specification).
    - `Exact`: Whitelist of explicit origin URLs (e.g. `https://app.example.com`).
    - `Wildcard`: Subdomain pattern matching (e.g. `https://*.example.com`).
    - `Custom`: Dynamic origin resolution predicate `Fn(&str) -> bool`.

3. **Method & Header Enforcement**:
    - `Access-Control-Allow-Methods`: Configurable list of permitted HTTP verbs.
    - `Access-Control-Allow-Headers`: Permitted request headers or reflection of `Access-Control-Request-Headers`.
    - `Access-Control-Expose-Headers`: Headers explicitly exposed to client-side scripts.
    - `Access-Control-Max-Age`: Caching duration for preflight responses in browser caches.
    - `Access-Control-Allow-Credentials`: Boolean flag indicating whether authorization headers and cookies are
      supported.

4. **Vary Header Management**:
    - Appends `Vary: Origin` and `Vary: Access-Control-Request-Method` / `Access-Control-Request-Headers` to prevent
      intermediate cache poisoning.
