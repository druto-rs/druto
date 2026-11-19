# `bodyparser` Crate Context & Architecture

## Vision & Scope

The `bodyparser` crate provides streaming request body decoding, multipart/form-data ingestion, temporary file spooling,
MIME-type detection, and request body size enforcement for the Druto web framework.

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/bodyparser` (streaming multipart parser, file upload lifecycle, size limiters)
- **Laravel**: `Illuminate\Http\UploadedFile` + request body parsing middleware
- **Loco**: `loco::controller::multipart`

## Key Capabilities & Responsibilities

1. **Streaming Multipart Parser**:
    - Chunked, non-blocking parsing of `multipart/form-data` streams without loading full file payloads into RAM.
    - Field extraction for mixed text fields and binary file streams.

2. **File Upload Abstraction (`UploadedFile`)**:
    - Encapsulates uploaded file state: original filename, client-supplied content type, detected MIME type, byte size,
      and disk location.
    - Helper methods for moving/persisting files to permanent storage directories or cloud disk stores.

3. **Magic-Byte MIME Sniffing**:
    - Inspects initial byte signatures to verify true MIME types independently of client-spoofed file extensions.

4. **Security & Quotas**:
    - Configurable maximum payload byte limits per request (e.g. 20MB for files, 1MB for JSON/form data).
    - Aborts parsing and yields `413 Payload Too Large` immediately when size limits are exceeded.
    - Automatic temporary file deletion on request drop or pipeline failure.
