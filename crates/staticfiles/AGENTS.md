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

