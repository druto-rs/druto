# `bodyparser` Agent Instructions

## Scope

This crate owns asynchronous onion middleware for request body decoding, multipart/form-data ingestion,
and payload quota enforcement for Druto, powered by the [`bojha`](https://github.com/Hitarashi/bojha) engine.

## Design Rules

1. **Powered by `bojha`**:
   - Do not implement custom body parsing logic or state machines inside this crate; all payload decoding, streaming multipart, temporary file spooling, MIME sniffing, and limits must be delegated to `bojha`.
   - Re-export `bojha::*` so downstream crates or consumers get a unified surface.

2. **Middleware Invariants**:
   - `BodyParserMiddleware` implements `dhap::Middleware<HttpContext, Response, RouterError>`.
   - Bypasses body decoding when `intended_method` is not in `allowed_methods`.
   - Bypasses requests with empty bodies or without a `Content-Type` header.
   - Respects `process_manually` route patterns for multipart streaming routes.
   - Populates `FormFields` and `UploadedFiles` into `ctx.extensions`.

3. **Framework Agnosticism & Clean Documentation**:
   - Follow Google's Rust Style Guide.
   - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`).
   - Avoid leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
   - Do not mention any other web frameworks in rustdocs, codebase comments, identifiers, or error messages.
   - Reference only RFCs, W3C standards, official specifications, and Druto types.

## Testing Requirements

- **Middleware Pipeline Execution**: Test parsing across JSON, URL-encoded form, multipart, and text payloads using `dhap::Pipeline`.
- **Payload Quota Rejection**: Assert immediate abort and `RouterError::PayloadTooLarge` when payload limits are exceeded.
- **Route-Level Bypass**: Verify manual multipart streaming bypass routes do not inject automatic extensions.
- **Method & Empty Body Bypass**: Verify `GET`, `HEAD`, and empty bodies pass through untouched.
