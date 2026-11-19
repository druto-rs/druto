# `validator` Agent Instructions

## Scope

This crate owns schema validation, input sanitization, built-in validation rules, custom validator registration, and
hierarchical error bag generation.

## Design Rules

1. **Deterministic & Safe Execution**:
    - Validation must fail closed: unvalidated fields must never pass through without explicit schema allowance.
    - Error messages must provide clear, actionable feedback without leaking internal system implementation details.

2. **Performance**:
    - Avoid unnecessary allocations during schema evaluation and string checks.
    - Compile regular expressions once or use pre-compiled regex tables.

3. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

4. **Framework-Agnostic Documentation & Codebase**:
    - Do not mention other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
    - Reference only web specifications and Druto types.
