# `staticfiles` Agent Instructions

## Scope

This crate owns static file serving from local filesystems, path traversal security, conditional HTTP cache negotiation
(ETag / 304), and byte-range requests.

## Design Rules

1. **Path Traversal Defense**:
    - Under no circumstances may requests access files outside the configured root directory.
    - Sanitize and canonicalize all URI paths before filesystem lookup.

2. **RFC Compliance**:
    - Conform strictly to RFC 7232 (Conditional Requests) and RFC 7233 (Range Requests).

3. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

4. **Framework-Agnostic Documentation & Codebase**:
    - Do not mention other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
    - Reference only RFCs, HTTP specifications, and Druto types.
