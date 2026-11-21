# `validator` Crate Context & Architecture

## Vision & Scope

The `validator` crate provides declarative schema definition, input sanitization, data coercion, validation rule
evaluation, and hierarchical field-level error reporting for the Druto web framework.

It operates across two distinct domains:

1. **Universal Data Validation (HTTP- & Platform-Agnostic)**:
    - Validates and decodes arbitrary in-memory data structures, `serde_json::Value`, `qs::QsValue`, hash maps, or
      primitives for background workers, CLI commands, configuration files, and domain services.
    - **Frontend & WebAssembly (WASM) Ready**: Core validation engine compiles cleanly to `wasm32-unknown-unknown`,
      allowing identical schemas to be shared isomorphically across the Rust backend and browser frontends.
2. **Multi-Source HTTP Request Validation**: Ingests, normalizes, and validates all parts of incoming HTTP requests:
    - **Body**: JSON payloads, URL-encoded forms, and multipart form fields.
    - **Query String**: URL search parameters (parsed via `qs` with bracket-notation nesting and string coercion).
    - **Route Parameters**: Dynamic URL path tokens (e.g. `:id`, `:slug`).
    - **Headers**: Custom and standard HTTP request headers (e.g. `X-Api-Key`, `Idempotency-Key`).
    - **Cookies**: Plain, signed, or encrypted cookies.

Architecturally, the crate is designed around seven core tenets:

- **Total Stack Ownership**: A self-contained, pure-Rust declarative schema validation engine without third-party
  validation wrappers.
- **"Parse, Don't Validate" & Direct Type Transformation**: Normalizes untrusted, weakly-typed input (trimming strings,
  empty-string-to-null conversion, query string type coercion) and validates it before decoding directly into strictly
  typed Rust data models, domain newtypes, and Rust enums via `.transform::<Out>()` and `schema::r#enum::<T>()`.
- **Fail-Closed Object Security by Default**: Object schemas default to a **`strip` policy**, silently discarding
  undeclared fields from the decoded output to eliminate mass-assignment vulnerabilities. Explicit `.strict()`
  (rejection with 422) and `.passthrough()` (retention) modifiers are available when required.
- **Thread-Safe, Immutable & Zero-Allocation Shared Execution**: Schema instances are immutable, cheaply cloneable,
  and `Send + Sync + 'static`. Validation methods take `&self`, allowing schemas to be declared once at startup (e.g. in
  `static LazyLock` singletons or router route definitions) and executed concurrently across worker threads
  with zero per-request allocation.
- **Ambient Runtime Metadata via `ValidationContext`**: Strongly-typed typemap container passed into validation calls
  (and auto-populated from `HttpContext::extensions` in HTTP handlers), enabling custom sync/async rules to inspect
  ambient request state (e.g. current user ID, tenant ID) for authorization and uniqueness checks.
- **Tri-Layer Authoring Ergonomics**:
    - **Layer 1: Fluent Schema Builders**: Composable, programmatic schema tree (`schema::object()`,
      `schema::string()`).
    - **Layer 2: Declarative Macro DSL**: Terse manifest DSL (`schema! { ... }`, `request_schema! { ... }`).
    - **Layer 3: Struct Derive Macro**: `#[derive(Validate)]` with field attributes targeting specific request sources
      (`#[validate(body)]`, `#[validate(query)]`, `#[validate(param)]`, `#[validate(header)]`).
- **Feature-Gated Dual-Mode Execution**:
    - Pure schemas execute synchronously with zero runtime overhead via `schema.validate(&input)`.
    - Asynchronous rules (`.refine_async(...)`, `AsyncRule`) are gated under an optional `async` feature flag, keeping
      the core engine portable and lightweight for client-side/WASM builds.

## Reference Blueprint & Specification Mapping

The crate synthesizes proven validation paradigms from established web standards and framework architectures:

- **Official Standards & Specifications**:
    - **RFC 9110**: HTTP Semantics:
        - Section 15.5.21: `422 Unprocessable Entity` for syntactic or semantic validation failures.
        - Section 8.6: `Content-Type` inspection for payload decoding.
    - **RFC 9457 / RFC 7807**: Problem Details for HTTP APIs (structured field-level invalid parameters).
    - **RFC 5322**: Internet Message Format (Section 3.4.1 email address syntax validation).
    - **RFC 4122 / RFC 9562**: Universally Unique IDentifiers (UUID v1–v8 format validation).
    - **RFC 3986**: Uniform Resource Identifier (URI) Generic Syntax.
    - **RFC 3339 / ISO 8601**: Date and Time formatting profiles.

## Feature Flags & Portability

The crate is organized into modular Cargo features:

| Feature    | Description                                                                                 | Default |
|------------|---------------------------------------------------------------------------------------------|---------|
| `async`    | Enables asynchronous validation execution (`validate_async`, `refine_async`, `AsyncRule`).  | Yes     |
| `macros`   | Enables declarative DSL macros (`schema!`, `request_schema!`).                              | Yes     |
| `derive`   | Enables the `#[derive(Validate)]` procedural macro for struct DTOs.                         | No      |
| `router`   | Integrates with `crates/router` (`HttpContext` extraction and automatic 422 JSON response). | No      |
| **(core)** | Core synchronous schema builder, built-in rules, sanitizers, and error bag (WASM-friendly). | Always  |

## Key Capabilities & Core Responsibilities

1. **Universal & Standalone Validation Engine**:
    - Pure, zero-IO core engine validating any in-memory data (`serde_json::Value`, `qs::QsValue`, scalars, vectors,
      maps).
    - Fully compatible with `wasm32-unknown-unknown` when building without the `router` feature.

2. **Multi-Source HTTP Request Validation**:
    - **Granular Single-Source APIs**:
      ```rust
      let body = ctx.validate_body(&user_schema).await?;
      let query = ctx.validate_query(&filter_schema)?;
      let params = ctx.validate_params(&param_schema)?;
      let headers = ctx.validate_headers(&header_schema)?;
      ```
    - **Unified Composite Request Schemas**:
      Validates query, params, headers, and body simultaneously in a single pass, aggregating all errors across all
      sources before returning `422 Unprocessable Entity`:
      ```rust
      let request_schema = schema::request()
          .params(param_schema)
          .query(filter_schema)
          .body(body_schema)
          .headers(header_schema);

      let form = ctx.validate(&request_schema).await?;
      ```

3. **Ambient Runtime State via `ValidationContext`**:
    - Strongly-typed typemap container passed via `validate_with_context(&data, &ctx)` or
      `validate_async_with_context(&data, &ctx).await`.
    - Automatically populated from `HttpContext::extensions` when executing inside route handlers.
    - Accessible within custom sync and async rule predicates (e.g. `ctx.get::<CurrentUserId>()` or
      `ctx.get::<TenantId>()`).

4. **Dual-Mode Execution & Native Async Rules (Feature-Gated)**:
    - **Synchronous Path**: `validate(&data)` executes pure rules synchronously with zero allocation or async task
      overhead.
    - **Asynchronous Path (`feature = "async"`)**: `validate_async(&data).await` executes schemas containing async
      predicates (`.refine_async(...)`, `AsyncRule`), allowing database checks (e.g. uniqueness) and external API
      validations to coexist with structural rules in a single error report.

5. **Tri-Layer Authoring Ergonomics**:
    - **Fluent Builder API**: Programmatic schema construction with rich method chaining.
    - **Declarative `schema!` & `request_schema!` Macros**: Terse, readable route-level and service-level schema
      definitions.
    - **`#[derive(Validate)]` Macro**: Automated schema synthesis from struct attributes with source mapping:
      ```rust
      #[derive(Debug, Validate)]
      struct UpdateUserRequest {
          #[validate(param, int)]
          id: u64,

          #[validate(query, min = 1, default = 1)]
          page: u32,

          #[validate(body, min_len = 3)]
          username: String,

          #[validate(header("x-idempotency-key"), uuid)]
          idempotency_key: Option<String>,
      }
      ```

6. **Type Transformation & Custom Domain Enums**:
    - `.transform::<Out>(|val| ...)`: Safely transforms validated primitives into custom domain newtypes or value
      objects (e.g. `UserId(u64)`, `PostalCode`), converting mapping errors into field-level `422` validation issues.
    - `schema::r#enum::<T>()`: Directly decodes input strings into strongly-typed Rust enums implementing `FromStr`,
      auto-generating informative mismatch errors (`"Expected one of: admin, member, guest"`).

7. **Fail-Closed Unknown Field Policies (Mass-Assignment Defense)**:
    - **`strip` (Default)**: Discards unvalidated fields from the decoded output struct or map, ensuring business logic
      never receives undeclared inputs.
    - **`.strict()`**: Rejects payloads containing any undeclared field with a `422 Unprocessable Entity` error
      (`rule: "unknown_field"`).
    - **`.passthrough()`**: Explicitly preserves extra fields in an untyped catch-all container when flexible payloads
      are required.

8. **First-Class Cross-Field Combinators & Conditional Rules**:
    - **Declarative Field Combinators**:
        - `.same_as("password")` (confirmation match).
        - `.different_from("old_password")` (field distinction).
        - `.required_if("country", "US")` / `.required_without("company_tax_id")` (conditional presence).
        - `.after_field("start_date")` / `.before_field("end_date")` (relative temporal/numeric comparisons).
        - `.when("payment_method", |val| val == "card", |schema| schema.min_len(16))` (conditional rule branches).
    - **Object-Level Refine Closures**: `.refine(...)` and `.refine_async(...)` for arbitrary multi-field validation
      calculations.

9. **Comprehensive Built-in Validation Rules**:
    - **String Rules**: `min_len`, `max_len`, `exact_len`, `email`, `url`, `uuid`, `ip` (v4/v6), `regex`, `alpha`,
      `alphanumeric`, `starts_with`, `ends_with`, `contains`, `ascii`.
    - **Numeric Rules**: `min`, `max`, `range`, `integer`, `positive`, `negative`, `multiple_of`.
    - **Date & Time Rules**: ISO 8601 / RFC 3339 format, `before`, `after`, `same_or_before`, `same_or_after`.
    - **Collection Rules**: `min_items`, `max_items`, `exact_items`, `not_empty`, `distinct`, `subset`.
    - **Boolean Rules**: `accepted` (truthy values: `"yes"`, `"on"`, `"1"`, `true`, `1`).

10. **Input Sanitization, Normalization & Coercion (Consumer UX & Form Safety)**:
    - Automatic or opt-in trimming of leading and trailing whitespace on string fields.
    - Empty-string normalization (`empty_string_as_null`), preventing integer/date deserialization failures on empty
      HTML form fields.
    - Safe string-to-number and string-to-boolean coercion for query strings, route params, and headers.

11. **Hierarchical Field-Level Error Bag & Execution Flow**:
    - Canonical dot and bracket notation paths (e.g. `user.addresses[0].postal_code`, or `query.page` for composite
      requests).
    - Standardized error report structure with dynamic placeholder interpolation (`:field`, `:min`, `:max`, `:value`).
    - Cascading message resolution:
        1. Inline rule message (`.with_message(...)`)
        2. Field + rule override (`email.email`)
        3. Field wildcard override (`email.*`)
        4. Global `MessagesProvider` rule default (`email`)
        5. Built-in engine fallback
    - **Exhaustive Evaluation by Default**: Validates all fields across the payload and evaluates all chained rules on
      each field to collect all failures.
    - **Opt-in `.bail(true)`**: Developers can explicitly opt into short-circuiting on a field or schema to stop
      evaluation after the first failure.

12. **HTTP Error Wire Formats & Automatic Content Negotiation**:
    - When validation fails in an HTTP context, Druto responds with `422 Unprocessable Entity` and negotiates the wire
      format based on the client's `Accept` header:
        - **Simple Errors Array (`Accept: application/json`)**:
          ```json
          {
            "errors": [
              {
                "field": "user.addresses[0].postal_code",
                "rule": "min_len",
                "message": "The postal_code field must be at least 5 characters long."
              }
            ]
          }
          ```
        - **RFC 9457 / RFC 7807 Problem Details (`Accept: application/problem+json`)**:
          ```json
          {
            "type": "https://druto.dev/errors/validation",
            "title": "Unprocessable Entity",
            "status": 422,
            "detail": "Validation failed for 1 field.",
            "invalid_params": [
              {
                "name": "user.addresses[0].postal_code",
                "reason": "The postal_code field must be at least 5 characters long.",
                "rule": "min_len"
              }
            ]
          }
          ```

13. **Router & Pipeline Integration**:
    - Direct integration with `router::Request` and `router::HttpContext`.
    - Route-level attachment via `.validate(schema)` / `.validate_body(schema)` / `.validate_query(schema)`.
    - Automatic generation of `422 Unprocessable Entity` responses with negotiated JSON bodies.

## Architectural Components & Data Flow

```
+---------------------------------------------------------------------------------------+
|                            Authoring Layer (Developer UX)                             |
|  ┌─────────────────────┐      ┌───────────────────────────┐   ┌────────────────────┐  |
|  │   Fluent Builder    │      │    Declarative Macros     │   │    Derive Macro    │  |
|  │  schema::object()   │      │ schema!{} request_schema! │   │ #[derive(Validate)]│  |
|  └──────────┬──────────┘      └─────────────┬─────────────┘   └─────────┬──────────┘  |
+─────────────┼───────────────────────────────┼───────────────────────────┼─────────────+
              └───────────────────────────────┼───────────────────────────┘
                                              ▼
+---------------------------------------------------------------------------------------+
|                          Multi-Source Request Ingestion                               |
|        Route Params         Query String           HTTP Headers         Request Body  |
|       (request.params)    (request.query via qs) (request.headers)   (JSON/Form/Multi)|
+─────────────┬───────────────────────┬─────────────────────┬───────────────────┬───────+
              │                       │                     │                   │
              ▼                       ▼                     ▼                   ▼
+---------------------------------------------------------------------------------------+
|                               Normalizer / Coercer                                    |
|  - Trims whitespace (opt-in / default)                                                |
|  - Normalizes empty strings to null ("" -> null)                                      |
|  - Coerces string-encoded values (params, query, headers) into target types           |
+---------------------------------------------------------------------------------------+
                                              │
                                              ▼
+---------------------------------------------------------------------------------------+
|                       Schema Evaluator & ValidationContext                            |
|  - Shared zero-allocation execution (Send + Sync + Clone, &self evaluation)           |
|  - Traverses schema tree across single source or composite multi-source               |
|  - Passes ValidationContext (current user, tenant) to custom sync & async rules       |
|  - Enforces fail-closed unknown-field policy (strip by default, strict, passthrough)  |
|  - Exhaustive rule evaluation by default (opt-in .bail(true) short-circuiting)        |
|  - Dual-mode execution: sync rules in validate(), async rules in validate_async()     |
|  - Resolves cross-field dependencies (.same_as, .required_if, .when)                  |
|  - Transforms validated primitives into domain types via .transform() & r#enum::<T>()|
|  - Resolves error messages via cascading priority                                     |
+---------------------------------------------------------------------------------------+
                  │                                                   │
         [Validation Passed]                                 [Validation Failed]
                  │                                                   │
                  ▼                                                   ▼
      +-----------------------+                           +-----------------------+
      | Typed Validated Model |                           | ValidationError (422) |
      | (Domain Model / DTO)  |                           | (Content Negotiated)  |
      +-----------------------+                           | application/json      |
                                                          | application/problem+json
                                                          +-----------------------+
```
