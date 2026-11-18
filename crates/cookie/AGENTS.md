# `cookie` Agent Instructions

## Scope

This crate owns RFC 6265 cookie modeling, serialization to `Set-Cookie`, parsing from `Cookie`, and cookie jar
management (plain, signed, and encrypted). It must remain decoupled from specific HTTP server implementations and
networking sockets.

## Design Rules

1. **Strict Name & Value Validation**:
    - Cookie names and values must be validated against RFC 6265 token rules (no control characters, whitespace,
      semicolons, or commas in unquoted values).
2. **Cryptographic Purpose Binding**:
    - All signed and encrypted cookies must bind their specific cookie name (`"cookie:<name>"`) into the HMAC signature
      or AEAD Additional Authenticated Data (AAD) to prevent cross-cookie substitution attacks.
3. **No Silent Decryption Panic**:
    - If a client alters an encrypted or signed cookie, parsing must fail gracefully returning `Err(CookieError::...)`
      rather than panicking.
4. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure code cleanly.
5. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
6. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **RFC 6265 Formatting**: Verify all attributes (`SameSite`, `HttpOnly`, `Secure`, `Partitioned`, `Expires`, `Max-Age`,
  `Domain`, `Path`) serialize correctly.
- **Header Parsing**: Verify parsing multiple cookies from a single `Cookie` header string, handling whitespace and
  special characters.
- **Tamper Detection**: Assert that bit-flipped signed or encrypted cookies are rejected with appropriate error types.
- **Substitution Defense**: Test that copying an encrypted payload from cookie `A` to cookie `B` fails decryption due to
  purpose mismatch.
- **Jar Mutation Tracking**: Verify that adding, updating, and removing cookies in `CookieJar` yields the exact expected
  `Set-Cookie` headers.
