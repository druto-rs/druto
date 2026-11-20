# `bodyparser` Agent Instructions

## Scope

This crate owns streaming request body ingestion, chunked `multipart/form-data` decoding, temporary file spooling,
MIME-type detection, payload size quota enforcement, and body parser middleware for Druto.

## Design Rules

1. **Total Stack Ownership & Runtime Agnosticism (BYOR)**:
    - Never wrap external multipart parser crates; implement the streaming boundary delimiter state machine internally.
    - Decouple the core parser engine from any single async runtime (such as Tokio or Smol). Ingest stream chunks using
      standard `futures_core::Stream<Item = Result<Bytes, E>>` abstractions so callers can bring their own runtime.

2. **Memory Safety & Temporary File RAII**:
    - Never buffer unbounded payloads or large files into memory; stream directly to disk or enforce strict size
      ceilings.
    - Temporary files must implement deterministic RAII cleanup on `Drop`. If a request is aborted, times out, or
      panics,
      unpersisted temporary files must not leak on disk.
    - File movement (`move_to`) must attempt atomic filesystem renames (`fs::rename`) and fall back to streaming copy +
      unlink only when moving across filesystem mount boundaries (`EXDEV`). Missing parent directories must be created
      automatically.

3. **Streaming & Boundary Invariants**:
    - Multipart parsing must be incremental and chunked, searching for boundary delimiters without loading full part
      bodies into memory.
    - Support both sequential and concurrent field extraction where feasible.
    - Nested form fields (e.g. `user[profile][name]`, `tags[]`) must parse into structured maps consistent with `qs`.

4. **Security & Quota Enforcement**:
    - Limits must be checked incrementally during stream ingestion: immediately terminate the stream with
      `413 Payload Too Large` as soon as bytes read exceed the configured threshold, without draining remaining bytes.
    - Separate system quotas from file validation: abort requests immediately on system-level breaches (`max_body_size`,
      `max_fields_count`), but defer file-level validation errors (MIME mismatch, extension, per-file size) to
      `UploadedFile::errors()`.
    - Client-supplied filenames must be sanitized to strip path traversal sequences (`../`, `..\`, null bytes, control
      characters).
    - Unfinished or malformed multipart streams must return `400 Bad Request`.

5. **MIME Verification & Magic Bytes**:
    - Never trust client-supplied `Content-Type` headers or file extensions alone for security checks.
    - Binary content types must be verified via magic-byte inspection on initial stream bytes (first 512 bytes).
    - Permissive fallback: unrecognized signatures must default to `application/octet-stream` rather than failing,
      allowing arbitrary binary uploads unless an explicit MIME whitelist is configured.

6. **Middleware Pipeline Integration**:
    - Honor `allowed_methods`: skip body parsing for `GET`, `HEAD`, `OPTIONS`, and requests with `Content-Length: 0`.
    - Provide route-level bypass (`process_manually`): allow designated routes to opt out of automatic multipart
      spooling so handlers can consume raw streaming bodies directly.
    - Parsed payload data, structured form fields, and uploaded files must be made accessible to downstream route
      handlers via `HttpContext` and `Request`.

## Testing Requirements

- **Streaming Multipart Parser**: Verify boundary detection, quoted/unquoted boundary parameters, multiline headers,
  CRLF variations, mixed text fields and binary file streams across chunk boundaries.
- **Runtime Agnosticism**: Test stream parsing using basic `futures_core::Stream` implementations independent of
  specific
  async runtime task runners.
- **Quota & Fail-Fast Rejection**: Assert immediate abort and `413 Payload Too Large` when global body limit, file size
  limit, or field count limit is breached.
- **Malformed & Truncated Payloads**: Verify graceful failure on truncated streams, missing boundaries, and invalid
  headers returning `400 Bad Request`.
- **TempFile RAII Cleanup**: Assert that temporary files are deleted upon dropping `TempFile` or `UploadedFile`, and
  remain intact only after calling `persist_to()` or `move_to()`.
- **Cross-Device Persistence**: Simulate cross-device moves (`EXDEV`) and verify that copy fallback succeeds, parent
  directories are created, and temporary source files are unlinked.
- **Magic Byte Sniffing**: Verify correct MIME detection across supported formats (JPEG, PNG, GIF, WebP, PDF, ZIP) and
  detect forged extensions (e.g. executable disguised as `.png`).
- **Path Traversal Sanitization**: Test malicious filenames (`../../../etc/passwd`, `C:\Windows\System32\cmd.exe`) and
  verify safe sanitized basenames.
- **Middleware Pipeline Execution**: Test parsing across JSON, URL-encoded form, multipart, and text payloads, as well
  as route-level manual streaming bypass.
