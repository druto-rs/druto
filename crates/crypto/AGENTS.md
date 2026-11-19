# `crypto` Agent Instructions

## Scope

This crate owns symmetric authenticated encryption (AEAD), HMAC signing, key rotation management, purpose binding,
and signed URL generation and validation. It must remain decoupled from HTTP server types, request contexts, and
password hashing.

## Design Rules

1. **Constant-Time Verification**:
    - All signature and authentication tag comparisons must execute in constant time (`subtle::ConstantTimeEq`) to
      prevent timing side-channel attacks. Never use standard `==` for cryptographic signatures or MAC tags.
2. **Cryptographically Secure Nonce Generation**:
    - Every AEAD encryption operation must generate a fresh 96-bit (12-byte) random nonce using a CSPRNG. Nonces must
      never be reused, static, or predictable.
3. **Strict Key Length Enforcement**:
    - Ensure keys satisfy exact 256-bit (32-byte) length requirements for AES-256-GCM and ChaCha20-Poly1305.
4. **Key Rotation Invariant**:
    - Encryption operations must strictly use the primary key. Decryption operations must try the primary key first,
      falling back to previous keys in sequence.
5. **Purpose Binding via Associated Data (AAD)**:
    - When purpose binding is used, the purpose string must be passed as authenticated associated data (AAD) to the AEAD
      cipher so any tampering or purpose mismatch immediately fails authentication.

## Testing Requirements

- **Known Answer Tests (KAT)**: Verify HMAC-SHA256 and cipher outputs against standard test vectors.
- **Roundtrip Encryption & Decryption**: Test that arbitrary plaintexts roundtrip cleanly for both `Aes256Gcm` and
  `ChaCha20Poly1305`.
- **Tamper Detection**: Assert that flipping any single bit in ciphertext, nonce, or tag cleanly fails decryption.
- **Key Rotation Verification**: Test that ciphertexts produced by an older primary key decrypt successfully using a
  `KeyRing` where the old key is placed in previous keys.
- **Purpose Binding Isolation**: Test that ciphertext encrypted with purpose A fails decryption when requested with
  purpose B.
- **Signed URL Expiration**: Test that expired signed URLs are rejected, while active ones pass validation.
