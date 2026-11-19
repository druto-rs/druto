# `staticfiles` Crate Context & Architecture

## Vision & Scope

The `staticfiles` crate provides high-performance, secure static asset delivery, client caching negotiations, and
HTTP range requests for the Druto web framework. It serves public frontend files (images, CSS, JavaScript, fonts,
media).

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/static` (public folder file server, ETag generation, range requests)
- **Django**: `django.contrib.staticfiles`
- **Tower / Axum**: `tower-http::services::ServeDir` + `ServeFile`

## Key Capabilities & Responsibilities

1. **Path Safety & Traversal Prevention**:
    - Strict path canonicalization preventing directory traversal attacks (`../`, null byte poisoning, hidden files).
    - Configurable symlink following policies.

2. **HTTP Caching & Conditional Requests**:
    - Automatic `ETag` computation based on file modification timestamps and inode/size.
    - Full support for `If-None-Match` and `If-Modified-Since` yielding immediate `304 Not Modified` responses.
    - Configurable `Cache-Control`, `Expires`, and `immutable` header directives.

3. **HTTP Range Requests (RFC 7233)**:
    - Byte-range streaming (`Range: bytes=start-end`) yielding `206 Partial Content` with `Content-Range` headers.
    - Essential for media playback (video/audio seek) and interrupted downloads.

4. **MIME-Type & Compression**:
    - Accurate MIME-type mapping per file extension with secure fallback defaults.
    - Pre-compressed file delivery (`.gz`, `.br`) when requested by client `Accept-Encoding`.
