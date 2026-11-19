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
