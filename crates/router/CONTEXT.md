# `router` Crate Context

## Purpose

`router` is the HTTP routing, request/response, and execution context layer for the Druto web framework. It bridges
`matcher` (radix trie route matching), `pipeline` (middleware execution), `cookie` (cookie jars), `qs` (nested query
parameter parsing), `crypto` (signed URLs), and `session` (session state) into a cohesive, developer-friendly, and
high-performance HTTP layer.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/http-server` (`Router`, `Route`, `RouteGroup`, `RouteResource`, `Request`, `Response`,
  `HttpContext`)
- **Laravel**: `illuminate/routing` (`Router`, `Route`, `RouteGroup`, `UrlGenerator`, `ControllerDispatcher`) &
  `illuminate/http` (`Request`, `Response`, `JsonResponse`, `RedirectResponse`)
- **Cot**: `cot::router` & `cot::request`
- **Topcoat**: `topcoat-router`

## Core Responsibilities

1. **Fluent Route Declaration & Hierarchy**:
    - Declarative route registration for HTTP methods: `get`, `post`, `put`, `patch`, `delete`, `head`, `options`, and
      `any`.
    - Route naming via `.name("route.name")` or `.as("route.name")` for reverse URL generation.
    - Nested route groups via `router.group(|group| { ... })`:
        - Path prefix inheritance (`.prefix("/api/v1")`).
        - Middleware inheritance and scoped stacking (`.middleware(...)`).
        - Route name prefixing (`.as("api.v1.")`).
    - Resource routing: `router.resource("posts", PostsController)` automatically expanding into conventional RESTful
      actions (`index`, `store`, `show`, `update`, `destroy`).

2. **Route Pattern Compilation & Dispatch Engine**:
    - Compiles declared routes into a radix trie via `matcher::MethodMatcher`.
    - Extracts typed, zero-allocation path parameters borrowed from the URI.
    - Automated RFC 9110 compliance:
        - Calculates and returns standard `405 Method Not Allowed` with `Allow` header when a path matches other verbs.
        - Automated `HEAD` fallback to `GET` handler when an explicit `HEAD` route is omitted.

3. **HTTP Request Representation**:
    - Strongly typed request abstraction containing:
        - HTTP verb (`Method`) and URI path.
        - Headers (`HeaderMap`) with typed accessors.
        - Parsed query parameters integrated with `qs::Qs`.
        - Request cookies integrated with `cookie::CookieJar`.
        - Raw or buffered request body payloads (JSON, form, text, bytes).
        - Client IP resolution and secure proxy headers evaluation.
    - Content negotiation helpers (`request.accepts()`, `request.is_json()`, `request.wants_json()`).
    - Cryptographic URL verification (`request.has_valid_signature()`) integrated with `crypto::Encrypter`.

4. **HTTP Response & Content Delivery**:
    - Fluent response builder for status codes, headers, and cookies.
    - Staging response cookies into the `CookieJar` (plain, signed, or encrypted).
    - Response body representations:
        - Text and HTML content with appropriate `Content-Type`.
        - JSON serialization via `serde_json`.
        - Redirects (`redirect.to_path(...)`, `redirect.to_route(...)`, `redirect.back()`).
        - File streaming and download responses with attachment headers.

5. **Unified HttpContext & Pipeline Onion**:
    - `struct HttpContext`: Request-scoped container holding `Request`, `Response`, `Session` (if active), route
      metadata,
      and custom request extensions/bag.
    - Seamless execution through `pipeline::Pipeline`: global middleware and route-level middleware execute in an
      asynchronous
      onion architecture before and after handler dispatch.

6. **Reverse Routing & URL Generation**:
    - Named route resolution: `router.url_for("users.show", &[("id", "42")])`.
    - Signed and temporary signed URL generation: `router.signed_url_for("verify.email", &params, ttl)`.

## Boundaries & Non-Responsibilities

- **Transport Agnostic**: Does not directly own raw TCP sockets or bind to a specific async server runtime (e.g. Hyper
  or Actix-net).
  It consumes abstract request data and produces abstract response data.
- **Not Authentication**: Delegates authentication checks and guards to `crates/auth`.
- **Not an ORM**: Does not query databases directly; controllers and route handlers interface with `crates/db`.

## Planned Public Surface

- `struct Router`: Main route registry and compiler.
- `struct Route`: Single route definition with method, pattern, handler, name, and middleware.
- `struct RouteGroup`: Scoped route container inheriting prefixes, names, and middleware.
- `struct Request`: Incoming HTTP request representation.
- `struct Response`: Outgoing HTTP response representation.
- `struct HttpContext`: Request-scoped execution context passed to handlers and middleware.
- `struct UrlGenerator`: Reverse URL resolution and cryptographic signing engine.
- `enum ResponseBody`: Encapsulated body payload (Empty, Bytes, Json, Stream).
- `enum RouterError`: Strongly typed errors (route conflict, route not found, method not allowed, invalid params).
