# `hash` Agent Instructions

## Scope

This crate owns password hashing, constant-time verification, multi-driver hashing management (`HashManager`),
automatic rehash detection (`needs_rehash`), and test doubles (`FakeHasher`). It must remain decoupled from database
ORMs,
user models, and reversible encryption.

## Design Rules

1. **Constant-Time Verification**:
    - Verification of password hashes must always execute in constant time (`subtle::ConstantTimeEq`) to eliminate
      timing side-channel attacks.
2. **Cryptographically Secure Salt Generation**:
    - Salts must always be generated using a CSPRNG. Every `make()` call must produce a uniquely salted hash even for
      identical
      passwords. Never reuse, predict, or truncate salts.
3. **Rehash Detection Invariant**:
    - `needs_rehash` must return `true` if:
        - The stored hash algorithm differs from the active hasher algorithm (e.g. legacy Bcrypt hash inspected by
          Argon2id hasher).
        - The stored parameters ($m, t, p$ for Argon2, or cost rounds for Bcrypt) are strictly lower than the hasher's
          current configuration.
        - The hash string format is malformed or unparseable.
    - `needs_rehash` must return `false` if the hash string meets or exceeds the active configuration.
4. **Driver Decoupling**:
    - Each driver (`Argon2Hasher`, `BcryptHasher`, `FakeHasher`) must independently implement the `Hasher` trait with
      its own
      strongly typed configuration struct.
5. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code or resolve the
      warning cleanly.
6. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
7. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **Roundtrip Verification**:
    - Test `make()` followed by `verify()` for valid passwords on `Argon2Hasher`, `BcryptHasher`, and `FakeHasher`.
- **Mismatch Rejection**:
    - Assert that incorrect passwords return `Ok(false)` rather than panicking or throwing an internal error.
- **Malformed Input Handling**:
    - Assert that malformed or truncated hash strings gracefully return a typed `HashError::InvalidHash` rather than
      crashing.
- **`needs_rehash` Scenarios**:
    - Verify that increasing memory cost, time cost, or rounds triggers `needs_rehash == true`.
    - Verify that matching parameters return `needs_rehash == false`.
    - Verify that cross-algorithm hashes (e.g., passing a Bcrypt hash to `Argon2Hasher::needs_rehash`) return `true`.
- **`FakeHasher` Performance**:
    - Verify that `FakeHasher` executes fast enough for dense integration test suites while accurately verifying matches
      and mismatches.
