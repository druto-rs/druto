# `crypto` Crate Context

## Purpose

`crypto` provides authenticated encryption (AEAD), message authentication code (HMAC) generation and verification,
key rotation management, purpose binding, and URL-safe payload envelopes for the Druto framework. It serves as the
cryptographic foundation for encrypted cookies, session identifiers, security tokens, and tamper-proof signed URLs.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@boringnode/encryption` (`Encryption`, `EncryptionManager`, `Hmac`) and
  `@adonisjs/core/modules/encryption`
- **Laravel**: `illuminate/encryption` (`Encrypter`, `KeyEncrypter`)
- **Underlying Standards**: Pure-Rust audited primitives from the `RustCrypto` project (`aes-gcm`, `chacha20poly1305`,
  `hmac`, `sha2`, `subtle`)

## Core Responsibilities

1. **Key Management & Seamless Rotation (`KeyRing`)**:
    - Holds a primary key (used for all new encryptions) and zero or more previous keys.
    - When decrypting, attempts the primary key first, then gracefully tries previous keys in order. This allows
      zero-downtime
      `APP_KEY` rotation in production environments without invalidating active user sessions.
    - Strict validation ensuring keys meet the required 256-bit (32-byte) length for AES-256 and ChaCha20.
2. **Authenticated Encryption with Associated Data (AEAD)**:
    - Modern authenticated ciphers providing both confidentiality and integrity:
        - **AES-256-GCM** (`Aes256Gcm`): Default driver, hardware-accelerated via AES-NI / ARMv8 Crypto Extensions.
        - **ChaCha20-Poly1305** (`ChaCha20Poly1305`): Pure software stream cipher alternative with high performance on
          platforms lacking hardware AES.
    - Unique 96-bit (12-byte) cryptographically secure random nonces per encryption operation. Nonces are never reused.
    - 128-bit authentication tags that verify ciphertext authenticity and reject tampered payloads before decryption.
3. **Purpose Binding (AAD - Additional Authenticated Data)**:
    - Supports binding a purpose or context string (e.g., `"cookie:session"`, `"password-reset"`) directly to ciphertext
      via AEAD associated data.
    - Payloads encrypted for one purpose cannot be decrypted or substituted under a different purpose, preventing
      cross-context replay attacks.
4. **URL-Safe Serialization Envelope**:
    - Encodes encrypted payloads into compact URL-safe base64 strings (`base64url` without padding).
    - Envelope format encapsulates algorithm identifier, nonce, authentication tag, and ciphertext.
5. **Message Authentication Codes (HMAC-SHA256)**:
    - Generates and verifies cryptographic signatures for raw payloads using HMAC-SHA256.
    - Verification strictly uses constant-time comparison (`subtle::ConstantTimeEq`) to eliminate timing side-channel
      attacks.
6. **Tamper-Proof & Temporary Signed URLs**:
    - Utilities to sign full URL paths and query strings.
    - Supports temporary signed URLs with UNIX expiration timestamps, automatically rejecting expired or altered
      requests.

## Boundaries & Non-Responsibilities

- **Not for Password Hashing**: Passwords must never use symmetric encryption. Password hashing requires memory-hard and
  compute-hard KDFs implemented in `crates/hash`.
- **Not HTTP-Coupled**: Does not depend on HTTP request/response types. It operates on strings, byte slices, and URLs,
  allowing clean reuse across cookies, sessions, cache, and CLI tokens.
- **No Hand-Rolled Math**: Relies on audited `RustCrypto` implementations under the hood rather than custom
  cryptographic math.

## Implemented Public Surface

- `struct Key`: 256-bit cryptographically verified key wrapper with CSPRNG generation, Base64 decoding, and redacted
  `Debug`.
- `struct KeyRing`: Primary key and fallback previous keys container for zero-downtime key rotation.
- `struct Encrypter`: High-level AEAD encrypter and decrypter supporting key rotation and context purpose binding.
- `struct Envelope`: Compact 29-byte binary header (`1B algorithm + 12B nonce + 16B auth tag + ciphertext`) encoded as
  unpadded URL-safe Base64 (`base64url`).
- `struct HmacSigner`: Timing-safe HMAC-SHA256 signer and verifier using constant-time comparison
  (`subtle::ConstantTimeEq`).
- `struct UrlSigner`: Canonical query string signer and validator for tamper-proof and temporary signed URLs with UNIX
  expiration.
- `enum Algorithm`: Supported ciphers (`Aes256Gcm = 1`, `ChaCha20Poly1305 = 2`).
- `enum CryptoError`: Strongly typed errors for key length violations, authentication tag mismatches, expired
  signatures, and purpose mismatches.
- Constants: `KEY_LEN` (32), `ALG_LEN` (1), `NONCE_LEN` (12), `TAG_LEN` (16), `HEADER_LEN` (29), `HMAC_SHA256_LEN` (32).
