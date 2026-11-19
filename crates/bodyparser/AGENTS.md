# `bodyparser` Agent Instructions

## Scope

This crate owns multipart/form-data streaming, file upload handling, temporary file lifecycle management, MIME type
validation, and payload size enforcement.

## Design Rules

1. **Memory Safety & Resource Protection**:
    - Never read entire unbounded file streams into memory buffers; always stream to disk or bounded chunks.
    - Guarantee automatic cleanup of temporary files created during request lifecycle, even in panic or abort
      conditions.

2. **MIME Verification**:
    - Never trust client-supplied `Content-Type` headers or file extensions alone; verify content via magic numbers.

3. **Style & Conventions**:
    - Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
    - Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
    - Code formatting must be verified via `cargo +nightly fmt --check`.

4. **Framework-Agnostic Documentation & Codebase**:
    - Do not mention other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
    - Reference only RFCs, MIME specifications, and Druto types.
