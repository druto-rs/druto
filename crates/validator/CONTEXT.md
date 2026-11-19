# `validator` Crate Context & Architecture

## Vision & Scope

The `validator` crate provides declarative request schema definition, input validation, sanitization, and structured
field-level error reporting for the Druto web framework. It bridges untrusted incoming HTTP payloads with strictly
typed Rust data structures.

## Reference Blueprint & Equivalent Mapping

- **AdonisJS / VineJS**: `vinejs` / `@adonisjs/validator` (compiler-backed fast schema validation, custom rules, field
  normalization)
- **Laravel**: `Illuminate\Validation\Validator` + `Illuminate\Foundation\Http\FormRequest`
- **TypeScript / Zod**: `zod` schema definition ergonomics adapted to idiomatic Rust

## Key Capabilities & Responsibilities

1. **Schema Builders**:
    - Declarative, composable schema builders for objects, primitives, arrays, records, and unions.
    - Type-safe decoding and coercion from `serde_json::Value`, query parameters, and form fields into validated domain
      models.

2. **Validation Rules**:
    - **String Rules**: `email`, `url`, `regex`, `alpha`, `alphanumeric`, `uuid`, `ip`, `min_len`, `max_len`, `trim`.
    - **Numeric Rules**: `min`, `max`, `range`, `integer`, `positive`, `negative`.
    - **Date & Time Rules**: ISO 8601 formatting, before/after comparisons.
    - **Collection Rules**: `not_empty`, `distinct`, `subset`, `length`.
    - **Custom Rules**: Extensible closure-based and trait-based validation predicates.

3. **Field-Level Error Formatting**:
    - Hierarchical field paths (e.g. `user.addresses[0].postal_code`).
    - Standardized JSON error response formatting (`errors: [{ field: "...", message: "...", rule: "..." }]`).
    - Custom error messages and parameter placeholder interpolation.

4. **Integration with `router`**:
    - Direct integration with `router::HttpContext` via extraction traits or validation middleware.
    - Automatic `422 Unprocessable Entity` responses when validation fails.
