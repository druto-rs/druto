# `cookie` Crate Context

## Purpose

`cookie` provides RFC 6265 compliant HTTP cookie modeling, parsing, serialization, and high-level cookie jars.
It provides three operational security modes—plain, tamper-proof signed, and confidential authenticated encrypted—by
directly consuming the `crypto` crate.

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
      format),
      `Secure`, `HttpOnly`, `SameSite` (`Strict`, `Lax`, `None`), and `Partitioned`.
    - String serialization to valid `Set-Cookie` header format.
    - Robust parsing of client `Cookie` request headers.
2. **Three Cookie Security Modes**:
    - **Plain Cookies**: Unmodified raw or JSON values readable and writable by clients when appropriate.
    - **Signed Cookies**: Tamper-proof cookies signed with HMAC-SHA256 via `crypto::HmacSigner`. Any client alteration
      invalidates the signature.
    - **Encrypted Cookies**: Confidential cookies encrypted via `crypto::Encrypter` (AEAD AES-256-GCM /
      ChaCha20-Poly1305). Values are opaque to clients and tamper-proof.
3. **Per-Cookie Purpose Binding**:
    - Cryptographically binds the cookie name as Additional Authenticated Data (`"cookie:<name>"`), preventing
      cross-cookie replay/substitution attacks (e.g. copying an encrypted value from `role` into `user_id`).
4. **`CookieJar` State & Mutation Tracking**:
    - Stores incoming request cookies.
    - Manages staged mutations for the outgoing response: setting new cookies, updating existing ones, and issuing
      removal/expiry headers (`Max-Age=0`).
    - Seamless integration with the framework's `KeyRing` for key rotation during cookie decryption.

## Boundaries & Non-Responsibilities

- **Not an HTTP Server**: Does not listen on sockets or handle TCP connections. Operates purely on headers and string
  representations.
- **Not Session Storage**: Cookie stores individual name-value pairs; session lifecycle, drivers, and flash messages
  belong in `crates/session`.

## Proposed Public Surface

- `struct Cookie`: Represents an individual RFC 6265 cookie with all attributes.
- `struct CookieBuilder`: Fluent builder for constructing cookies.
- `enum SameSite`: `Strict`, `Lax`, `None`.
- `struct CookieJar`: Request cookie accessor and outgoing response cookie staging area.
- `enum CookieError`: Strongly typed errors for invalid cookie names/values, decryption failures, signature mismatches,
  and malformed header strings.
