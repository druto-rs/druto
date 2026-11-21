# `bodyparser` Crate Context & Architecture

## Vision & Scope

The `bodyparser` crate provides asynchronous onion middleware for request body decoding, multipart/form-data ingestion,
and payload size quota enforcement for the Druto web framework, powered by the [`bojha`](https://github.com/Hitarashi/bojha) engine.

Architecturally, the crate acts as Druto's HTTP middleware layer:

1. **Powered by `bojha`**: Re-exports all core body parsing primitives, quota limits, file spooling, MIME sniffing, and
   data structures from the standalone `bojha` engine.
2. **Onion Pipeline Integration**: Implements `dhap::Middleware<router::HttpContext, router::Response, router::RouterError>`
   as `BodyParserMiddleware`.
3. **Automatic Context Extensions**: Automatically decodes incoming request bodies according to `Content-Type` headers
   and populates `ctx.extensions` with extracted `FormFields` and `UploadedFiles` for downstream route handlers.

## Reference Blueprint & Specification Mapping

The crate adheres to official web standards and Druto core specifications:

- **RFC 7578**: Returning Values from Forms: `multipart/form-data`.
- **RFC 9110**: HTTP Semantics:
  - Section 8.6: `Content-Type` header inspection and media type negotiation.
  - Section 15.5.14: `413 Content Too Large` (Payload Too Large).
  - Section 15.5.16: `415 Unsupported Media Type`.
- **RFC 2046**: Multipurpose Internet Mail Extensions (MIME) Part Two: Media Types.

## Key Capabilities & Core Responsibilities

1. **Asynchronous Onion Middleware (`BodyParserMiddleware`)**:
   - Inspects `ctx.request.intended_method()` against configured `allowed_methods` (defaults to `POST`, `PUT`, `PATCH`, `DELETE`, `QUERY`). Bypasses `GET`, `HEAD`, `OPTIONS`.
   - Bypasses requests with empty bodies.
   - Inspects `Content-Type` header:
     - `application/json` & vendor JSON subtypes (`application/*+json`): Decodes and validates payload against configured limits.
     - `application/x-www-form-urlencoded`: Decodes structured bracket-notation fields and populates `FormFields` into `ctx.extensions`.
     - `multipart/form-data`: Extracts boundary, parses text fields and spools files to temporary disk storage, populating `FormFields` and `UploadedFiles` into `ctx.extensions`.
     - Route-level bypass: Allows routes configured via `process_manually` to bypass automatic multipart file spooling so handlers can stream directly.
   - Propagates errors seamlessly as `RouterError::PayloadTooLarge`, `RouterError::DeserializationError`, or `RouterError::Internal`.

2. **Re-Export Surface**:
   - Re-exports all `bojha` primitives (`BodyParserConfig`, `BodyParserConfigBuilder`, `PayloadLimits`, `PayloadLimitsBuilder`, `UploadedFile`, `TempFile`, `FileValidator`, `FormFields`, `UploadedFiles`, `MimeSniffer`, `ParsedBody`, `parse`, `parse_stream`, `parse_from_headers`).
