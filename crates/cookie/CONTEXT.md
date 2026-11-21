# `cookie` Crate Context

## Purpose

`cookie` provides RFC 6265 compliant HTTP cookie modeling, parsing, serialization, and high-level cookie jars.
It provides three operational security modes—plain, tamper-proof signed, and confidential authenticated encrypted—by
directly consuming the `gopon` library.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/http-server/src/cookies` (`drivers/plain.ts`, `drivers/signed.ts`, `drivers/encrypted.ts`,
  `parser.ts`, `serializer.ts`)
- **Laravel**: `illuminate/cookie` (`CookieJar`, `EncrypterCookieStore`)
- **Topcoat**: `topcoat-cookie`
- **Underlying Standards**: RFC 6265bis (HTTP State Management Mechanism), RFC 6265 Partitioned Cookies (CHIPS)

## Core Responsibilities

1. **RFC 6265 Cookie Modeling & Serialization**:
    - Comprehensive attribute support: `Name`, `Value`, `Path`, `Domain`, `Max-Age`, `Expires` (RFC 2822 / IMF-fixdate
      format via `time::OffsetDateTime`), `Secure`, `HttpOnly`, `SameSite` (`Strict`, `Lax`, `None`), and `Partitioned`
      (CHIPS).
    - String serialization to valid `Set-Cookie` header format with secure-by-default baselines (`Path=/`,
      `HttpOnly=true`, `SameSite=Lax`).
    - Robust, tolerant parsing of client `Cookie` request headers.
2. **Three Cookie Security Modes**:
    - **Plain Cookies**: Raw or URL-safe string values readable and writable by clients when appropriate.
    - **Signed Cookies**: Tamper-proof cookies prefixed with `s:` (`s:<value>.<signature_base64url>`) signed with
      HMAC-SHA256 via `gopon::HmacSigner`. Any client alteration invalidates the signature.
    - **Encrypted Cookies**: Confidential cookies prefixed with `e:` (`e:<crypto_envelope_base64url>`) encrypted via
      `gopon::Encrypter` (AEAD AES-256-GCM / ChaCha20-Poly1305). Values are opaque to clients and tamper-proof.
3. **Per-Cookie Purpose Binding**:
    - Cryptographically binds the cookie name (`"cookie:<name>"`) into AEAD Additional Authenticated Data (AAD) and HMAC
      signing inputs, preventing cross-cookie replay/substitution attacks (e.g. copying an encrypted value from `role`
      into `user_id`).
4. **`CookieJar` Overlay State & Delta Mutation Tracking**:
    - Parses incoming request cookies from `Cookie` headers.
    - Implements an overlay mutation model enabling read-your-own-writes across onion middleware layers.
    - Separately tracks the mutation delta (`outgoing_cookies()` / `delta()`) so only added, modified, or removed
      (`Max-Age=0`) cookies produce outgoing `Set-Cookie` headers.
    - Seamless integration with `gopon::KeyRing` for zero-downtime key rotation during cookie decryption.

## Boundaries & Non-Responsibilities

- **Not an HTTP Server**: Does not listen on sockets or handle TCP connections. Operates purely on headers and string
  representations.
- **Not Session Storage**: Cookie stores individual name-value pairs; session lifecycle, drivers, and flash messages
  belong in `crates/session`.

## Implemented Public Surface

- `struct Cookie`: Represents an individual RFC 6265 compliant cookie with token validation, full attribute support
  (`name`, `value`, `path`, `domain`, `expires`, `max_age`, `secure`, `http_only`, `same_site`, `partitioned`), deletion
  tombstone detection (`is_tombstone()`), `to_set_cookie_header()`, and request/response header parsing
  (`parse_cookie_header`, `parse_set_cookie`).
- `struct CookieBuilder`: Fluent builder for constructing validated cookies with custom attributes.
- `enum SameSite`: `Strict`, `Lax`, `None` with RFC 6265 case-insensitive parsing and formatting.
- `struct CookieJar`: Request cookie accessor and outgoing response cookie staging area with read-your-own-writes
  overlay and mutation delta tracking (`delta()`, `outgoing_cookies()`, `render_set_cookie_headers()`, `unstage()`,
  `remove_with_options()`).
- `struct CookieConfig`: Default attribute policy configuration with secure-by-default baselines (`default_path="/"`,
  `default_http_only=true`, `default_same_site=Some(SameSite::Lax)`).
- `enum CookieSecurity`: `Plain`, `Signed`, `Encrypted` with wire prefix detection (`s:` and `e:`).
- `enum CookieError`: Strongly typed errors for invalid cookie names/values, decryption failures, signature mismatches,
  missing keys, and malformed header strings.
- `security` module: Low-level functions `pack_signed`, `unpack_signed`, `unpack_signed_with_keyring`, `pack_encrypted`,
  `unpack_encrypted`, and `unpack_encrypted_with_keyring` with purpose binding `"cookie:<name>"`.
