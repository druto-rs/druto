# `bodyparser` Crate Context & Architecture

## Vision & Scope

The `bodyparser` crate provides streaming request body decoding, chunked `multipart/form-data` ingestion, temporary file
spooling, magic-byte MIME type detection, payload size quota enforcement, and asynchronous onion middleware for the
Druto
web framework.

Architecturally, the crate is designed around two core tenets:

1. **Total Stack Ownership**: A self-contained, pure-Rust streaming multipart parser state machine without third-party
   parsing dependencies.
2. **Runtime Agnostic (Bring Your Own Runtime - BYOR)**: Core streaming state machines, chunk decoders, and
   byte-counting
   guards operate over standard asynchronous traits (`futures_core::Stream`, generic byte streams) rather than binding
   exclusively to a specific async executor (such as Tokio or Smol).

It operates across two complementary layers:

- **Core Parser Engine (HTTP- & Runtime-Agnostic)**: Standalone streaming multipart, URL-encoded form, JSON, and raw
  text
  parsers, `UploadedFile` lifecycle primitives, magic-byte MIME sniffer, and byte-counting limit guards.
- **HTTP Middleware Integration**: `BodyParserMiddleware` integrating with `dhap` and `router`, automatically
  inspecting incoming `Content-Type` headers, streaming payloads to memory or temporary disk storage, and exposing
  structured fields and uploaded files to downstream handlers.

## Reference Blueprint & Specification Mapping

The crate synthesizes proven body ingestion architectures from framework designs and official web standards:

- **Streaming Multipart & Upload Lifecycle Blueprint**:
    - **AdonisJS**: `@adonisjs/bodyparser` & `@poppinss/multiparty` (in-house streaming multipart parser, automatic
      tempfile spooling vs manual stream routing, deferred file validation, `qs` field structuring).
    - **Laravel**: `Illuminate\Http\UploadedFile` (file movement, original metadata preservation, size validation).
    - **Rocket**: `rocket::data::Limits` & `rocket::fs::TempFile` (hierarchical byte limits, RAII temporary file cleanup
      on
      drop).
    - **Actix-Web**: `actix-multipart` (in-house boundary scanning and stream state machine).
- **Official Standards & Specifications**:
    - **RFC 7578**: Returning Values from Forms: `multipart/form-data` (boundary parsing, preamble/epilogue handling,
      `Content-Disposition` header parameters, nested field names).
    - **RFC 9110**: HTTP Semantics:
        - Section 8.6: `Content-Type` header inspection and media type negotiation.
        - Section 15.5.14: `413 Content Too Large` (Payload Too Large).
        - Section 15.5.16: `415 Unsupported Media Type`.
    - **RFC 2046**: Multipurpose Internet Mail Extensions (MIME) Part Two: Media Types.
    - **Shared MIME-Info Database**: Magic-byte signature sniffing for reliable content verification independent of
      client
      file extensions.

## Key Capabilities & Core Responsibilities

1. **Self-Contained Streaming Multipart Parser (`Multipart`)**:
    - Incremental, non-blocking chunked parsing of `multipart/form-data` streams without buffering full payloads into
      RAM.
    - Handles mixed text fields and binary file streams across boundary transitions.
    - Uses Druto's `qs` bracket-notation parser to structure multipart text fields into nested maps and vectors (e.g.
      `users[0][name]`, `items[]`), providing complete parity with URL-encoded forms.

2. **Runtime-Agnostic Streaming Core (BYOR)**:
    - Decoupled from runtime-specific task or thread primitives.
    - Ingests byte chunks via standard `futures_core::Stream<Item = Result<Bytes, E>>` interfaces, enabling execution
      across
      Tokio, Smol, async-std, or custom embedded async runtimes.

3. **Temporary File Spooling & RAII Cleanup (`TempFile`)**:
    - Streams incoming binary file parts directly to temporary files on disk when payload exceeds memory thresholds or
      when configured for automatic processing.
    - Uses OS temporary directory or custom configured directory with unique, collision-proof UUID filenames.
    - Implements strict RAII cleanup: temporary files are deleted on `Drop` unless explicitly persisted via
      `persist_to()` or `copy_to()`.

4. **Uploaded File Abstraction & Cross-Device Move (`UploadedFile`)**:
    - Encapsulates uploaded file state: field name, client original filename, client-declared Content-Type, detected
      MIME type, byte size, file extension, and temporary disk path.
    - Tracks validation state (`Validated`, `ExceededLimit`, `InvalidExtension`, `InvalidMimeType`).
    - **Resilient Persistence (`move_to`)**: Attempts an atomic filesystem rename first. If crossing filesystem
      boundaries (triggering OS `EXDEV`), seamlessly falls back to streaming async copy plus tempfile deletion.
      Automatically creates
      missing parent directories and disarms the `TempFile` RAII deletion on drop.

5. **Magic-Byte MIME Sniffing (`MimeSniffer`)**:
    - Built-in, zero-dependency pure-Rust signature table inspecting initial bytes (first 512 bytes).
    - Detects standard web formats: JPEG, PNG, GIF, WebP, SVG, PDF, ZIP, GZIP, MP3, MP4.
    - Permissive fallback: unrecognized binary formats fall back to `application/octet-stream` rather than failing,
      allowing arbitrary binary uploads unless an explicit whitelist is applied.

6. **Hierarchical Security Quotas & Fail-Fast Defense (`PayloadLimits`)**:
    - Secure-by-default baselines:
        - `max_body_size`: 20 MB (global request payload ceiling).
        - `max_file_size`: 10 MB (per individual uploaded file).
        - `max_fields_size`: 1 MB (aggregate size of all text fields combined).
        - `max_fields_count`: 1,000 (maximum number of allowed fields).
        - `max_depth`: 100 (maximum container nesting depth for JSON and form inputs).
    - **Validation Timing Separation**: System-level quota breaches abort the stream immediately with
      `413 Payload Too Large`
      or `400 Bad Request`. File-specific validation failures (MIME mismatch, extension whitelist, per-file size) are
      deferred onto `UploadedFile::errors()` for form-level feedback.
    - Path traversal defense: sanitizes client-supplied filenames (`../`, null bytes, control characters).

7. **Middleware Integration (`BodyParserMiddleware`)**:
    - Inspects `Content-Type` and routes to the appropriate parser:
        - `application/json`: Parsed with nesting depth and size limits.
        - `application/x-www-form-urlencoded`: Parsed via `qs` with depth and size limits.
        - `multipart/form-data`: Handled via `Multipart` parser with automatic spooling or manual streaming.
        - `text/*`: Ingested as raw UTF-8 string with size limits.
    - Route-level bypass (`process_manually`): Allows designated routes (e.g. streaming file proxy to object stores) to
      opt out of automatic disk spooling and consume the raw stream directly.

## Boundaries & Non-Responsibilities

- **Not Cloud Object Storage**: Persists files locally; integration with cloud object stores (S3, GCS, Azure) belongs to
  storage/drive crates.
- **Not Image/Media Processing**: Does not perform image transformation, resizing, or transcoding.
- **Not Domain Validation**: Validates file mechanics (size, MIME type, extension); domain schema validation belongs to
  `validator`.

## Implemented Public Surface

- `struct MultipartParser`: Self-contained streaming multipart boundary state machine with RFC 7578 / RFC 2046
  compliance.
- `fn parse_multipart_stream`: Runtime-agnostic asynchronous stream parser over generic `futures_core::Stream`.
- `fn extract_boundary`: Boundary parameter extractor from `Content-Type` headers.
- `enum MultipartField`: Individual field extracted from a multipart payload (`Text` value or `File`).
- `struct ParsedMultipart`: Collection of parsed text fields and uploaded files, with bracket-notation `qs` tree
  extraction.
- `struct UploadedFile`: High-level uploaded file representation with metadata, path traversal sanitization, and
  resilient `move_to` persistence.
- `struct TempFile`: RAII managed temporary file handle with automatic deletion on drop unless persisted.
- `struct PayloadLimits` & `struct PayloadLimitsBuilder`: Hierarchical payload size and field count configurations.
- `struct BodyParserConfig` & `struct BodyParserConfigBuilder`: Configuration options for JSON, form, text, and
  multipart parsers.
- `struct BodyParserMiddleware`: Asynchronous onion middleware implementing
  `dhap::Middleware<HttpContext, Response, RouterError>`.
- `struct FileValidator`: Fluent rules for allowed extensions, MIME types, and size bounds.
- `enum FileError`: Granular file-level validation errors (`SizeLimitExceeded`, `InvalidExtension`, `InvalidMimeType`,
  `MimeMismatch`).
- `struct MimeSniffer`: Pure-Rust magic-byte signature detector for images, documents, audio/video, and archives.
- `struct UploadedFiles` & `struct FormFields`: Request-scoped containers stored in `HttpContext::extensions`.
- `enum BodyParserError`: Strongly typed errors for size violations, malformed boundaries, invalid MIME, and I/O
  failures.
- `parsers` module: `parse_form`, `parse_form_value`, `parse_json`, `parse_json_value`, `parse_text` functions.

## Dependencies & Integration Points

- `bytes`: Chunk buffer management.
- `futures-core` / `futures-util`: Runtime-agnostic async stream abstractions.
- `tempfile`: Underlying temporary file allocation.
- `qs`: Bracket-notation form and field query parsing.
- `serde` / `serde_json`: JSON decoding.
- `router`: Integration with `Request`, `Response`, and `HttpContext`.
- `pipeline`: Integration with middleware pipeline.
