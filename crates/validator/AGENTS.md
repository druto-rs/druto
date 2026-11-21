# `validator` Agent Instructions

## Scope

This crate owns universal schema definition, multi-source HTTP request validation (body, query string, route params,
headers, cookies), input sanitization, data coercion, validation rule evaluation, custom rule extensibility (sync and
async), cross-field relationship resolution, ambient runtime context passing, domain type transformation, hierarchical
error bag generation, and HTTP extraction integration for Druto.

## Design Rules

1. **Universal & Decoupled Core**:
    - The core schema validation engine must remain strictly decoupled from HTTP sockets and async runtimes.
    - Any in-memory data (`serde_json::Value`, `qs::QsValue`, maps, or primitive scalars) must be validatable in
      isolation without an HTTP request context.
    - The core engine (without `router` or `async` features) must compile cleanly to `wasm32-unknown-unknown` for
      client-side/frontend portability.

2. **Thread Safety & Immutability**:
    - Schema instances must be immutable, cheaply cloneable, and `Send + Sync + 'static`.
    - Evaluation methods (`validate`, `validate_async`, `validate_with_context`, etc.) must take `&self` to allow
      zero-allocation
      concurrent execution across threads from `static LazyLock` singletons or router route definitions.

3. **Modular Feature Gating**:
    - Gate async execution (`validate_async`, `refine_async`, `AsyncCustomRule`) behind `feature = "async"`.
    - Gate HTTP router integration behind `feature = "router"`.
    - Gate declarative macros behind `feature = "macros"`.
    - Gate struct derive macros behind `feature = "derive"`.

4. **Dual-Mode Execution (Sync & Native Async)**:
    - Pure schemas must evaluate synchronously via `validate(&self, ...)` with zero async task overhead.
    - Schemas containing asynchronous predicates (`refine_async`, `AsyncCustomRule`) must evaluate via
      `validate_async(&self, ...).await`.
    - Both execution paths must produce the identical, unified `ValidationError` structure on failure.

5. **Ambient Runtime State via `ValidationContext`**:
    - Provide a typemap-backed `ValidationContext` passed into `validate_with_context` and
      `validate_async_with_context`.
    - Route handler integration must automatically project `HttpContext::extensions` into the `ValidationContext`.
    - Custom sync and async predicates must receive read-only access to this context to execute domain authorization
      and uniqueness checks (e.g. `ctx.get::<CurrentUserId>()`).

6. **Fail-Closed Unknown Field Invariants**:
    - Object schemas must default to `strip` mode: unvalidated/unexpected fields must be omitted from the output model
      to prevent mass-assignment vulnerabilities.
    - In `strict` mode, any unvalidated field must immediately register an `unknown_field` validation error.
    - Extra fields are retained only when `.passthrough()` is explicitly declared.

7. **Type Transformation & Enums**:
    - `.transform::<Out>(|val| ...)` must map successfully validated values into domain types `Out`, automatically
      capturing transformation errors and mapping them to `ValidationError` entries at the exact field path.
    - `schema::r#enum::<T>()` must validate string values against variants of `T: FromStr`, generating clear
      `"Expected one of: ..."` error feedback on mismatch.

8. **Cross-Field Relationship Evaluation**:
    - Cross-field rules (`.same_as()`, `.required_if()`, `.after_field()`, `.when()`) must evaluate against the parent
      object's normalized field mapping during object traversal.
    - Field-level failures resulting from cross-field checks must attribute their error to the target field path with
      accurate parameter placeholders.

9. **Multi-Source Ingestion & Coercion**:
    - Support both dedicated single-source validation (`validate_body`, `validate_query`, `validate_params`,
      `validate_headers`) and composite multi-source request validation (`RequestSchema`).
    - String-encoded primitives originating from query strings, route parameters, and headers must be safely coerced
      into numbers, booleans, or parsed types before rule predicates execute.
    - Normalization (trimming, empty-string-to-null) must execute deterministically across all incoming sources.

10. **Enterprise UX & Error Recovery**:
    - **Exhaustive Evaluation by Default**: Evaluate all fields across the payload and execute all chained rules on
      each field to collect all failures.
    - **Opt-in Short-Circuiting**: Provide `.bail(true)` at both field and schema levels so developers can explicitly
      opt into short-circuiting after the first rule or field failure.
    - When composite requests fail, field error paths must clearly namespace the source where appropriate (e.g.
      `query.page` vs `body.username`).
    - Cascading message resolution must consistently evaluate:
      Inline Rule Override > Field+Rule Override > Field Wildcard > MessagesProvider Default > Engine Fallback.
    - Interpolate error parameters accurately (`:field`, `:min`, `:max`, `:value`).

11. **Performance & Memory Hygiene**:
    - Avoid unnecessary allocations during schema evaluation and string checks.
    - Pre-compile regular expressions once via static tables or lazy initializers (`std::sync::LazyLock`); never
      compile regexes per request or per validation call.
    - Field path construction must avoid heap allocations on the happy path; construct hierarchical path strings only
      when reporting an error.

12. **HTTP Standards & Error Invariants**:
    - Failed validations in HTTP contexts must yield `422 Unprocessable Entity` responses.
    - Support content negotiation on 422 responses:
        - Default / `Accept: application/json` returns Simple Errors Array (`{"errors": [...]}`).
        - `Accept: application/problem+json` returns RFC 9457 Problem Details
          (`{"type": "...", "status": 422, "invalid_params": [...]}`).

## Testing Requirements

- **Universal Core Tests**: Test schema validation over generic `serde_json::Value`, `qs::QsValue`, and native memory
  types without initializing HTTP servers.
- **Thread-Safety & Concurrency Tests**: Assert that schema instances are `Send + Sync + Clone` and can execute
  validation concurrently across threads without contention.
- **WASM Compatibility Verification**: Ensure the core crate compiles cleanly for `wasm32-unknown-unknown` with
  default features disabled.
- **ValidationContext Tests**: Verify that custom sync/async rules correctly access ambient typemap state and that
  `HttpContext::extensions` forward properly during request extraction.
- **Dual-Mode Execution Tests**: Verify synchronous evaluation for pure schemas and async execution for schemas with
  async predicates, ensuring identical error formatting.
- **Fail-Closed Security Tests**: Assert that unexpected fields are omitted by default (`strip`), rejected with 422 in
  `.strict()`, and preserved in `.passthrough()`.
- **Type Transformation & Enum Tests**: Test `.transform()` with valid and invalid mappings, and test
  `schema::r#enum::<T>()` with matching and non-matching variants.
- **Content Negotiation Tests**: Verify that 422 errors render RFC 9457 when `Accept: application/problem+json` is
  present and Simple Errors Array when `Accept: application/json` is present.
- **Cross-Field Tests**: Verify `.same_as()`, `.required_if()`, `.different_from()`, `.after_field()`, and `.when()`
  rules against matching, non-matching, and missing counterpart values.
- **Multi-Source Request Tests**: Test validation across request body, query string, route params, headers, and
  composite multi-source requests.
- **Primitive Rule Unit Tests**: Exhaustive coverage of edge cases for all primitive rules (strings, numbers, booleans,
  dates, collections).
- **Authoring Surface Parity**: Verify that schemas declared via Fluent Builders, `schema!` / `request_schema!`
  Macros, and `#[derive(Validate)]` produce identical validation and error outcomes.
- **Nested Object & Array Pathing**: Verify that errors occurring deep within nested structures accurately format their
  hierarchical field paths.
- **Sanitization & Coercion**: Test whitespace trimming, empty-string-to-null conversion, and string-to-number/bool
  coercion across query strings, params, and headers.
- **Custom Rule & Refine Closures**: Verify that custom sync/async closures and reusable rule traits execute and report
  errors identically to built-in rules.
- **Bail Opt-in Tests**: Assert exhaustive rule reporting by default, and verify that adding `.bail(true)` halts further
  rule evaluation on that field upon failure.
- **Cascading Message Overrides**: Verify priority ordering among inline rule messages, schema-level bags, and global
  providers with parameter interpolation.
- **Property-Based Testing**: Use `proptest` to test arbitrary strings, numbers, and collections against rule
  invariants.
