# `hash` Crate Context

## Purpose

`hash` provides secure, memory-hard password hashing and verification for the Druto framework. It provides a
multi-driver manager (`HashManager`), automatic rehash detection (`needs_rehash`), standard PHC string formatting,
and a blazing-fast test double (`FakeHasher`) for instantaneous test execution.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/hash` (`HashManager`, `Argon`, `Bcrypt`) and `@adonisjs/core/modules/hash`
- **Laravel**: `illuminate/hashing` (`HashManager`, `Argon2IdHasher`, `BcryptHasher`, `Hash::fake()`)
- **Underlying Standards**: Audited implementations from `argon2` (RFC 9106, PHC string format) and `bcrypt` (Modular
  Crypt Format)

## Core Responsibilities

1. **Unified `Hasher` Abstraction**:
    - Generic contract defining the core hashing operations:
        - `make(password: &str) -> Result<String, HashError>`: Generates a secure, salted hash string.
        - `verify(password: &str, hashed_value: &str) -> Result<bool, HashError>`: Constant-time password verification.
        - `needs_rehash(hashed_value: &str) -> bool`: Checks if hash cost parameters or algorithm are outdated.
2. **`HashManager` Multi-Driver Architecture**:
    - Central registry holding configured hashing drivers.
    - Resolves a default hasher (defaults to `Argon2id`), while allowing ad-hoc driver switching
      (`manager.use_driver("bcrypt")`).
3. **Argon2id Driver (Primary / Recommended)**:
    - Pure Rust implementation using `argon2` (v0.6.0).
    - OWASP-recommended algorithm for password storage (Argon2id v0x13).
    - Configurable memory cost ($m$), time cost ($t$), and parallelism ($p$).
    - Standard PHC format serialization (`$argon2id$v=19$m=65536,t=3,p=4$...`).
4. **Bcrypt Driver (Legacy & Cross-System Compatibility)**:
    - Pure Rust implementation using `bcrypt` (v0.19.3).
    - Configurable cost (rounds, 4 to 31; default 10–12).
    - Standard Modular Crypt Format serialization (`$2b$12$...`).
5. **Automatic Rehash Detection (`needs_rehash`)**:
    - Parses the cost parameters embedded in a stored hash string and compares them against the application's current
      configuration.
    - Detects if an algorithm has been upgraded (e.g. migrating users from Bcrypt to Argon2id) or if cost parameters
      have increased.
    - Enables authentication flows to transparently rehash and save the new password hash upon successful user login.
6. **`FakeHasher` for Blazing-Fast Tests**:
    - Argon2id is intentionally slow (100–300ms per verification). A test suite with hundreds of authentication tests
      would
      waste minutes spinning CPU.
    - `FakeHasher` provides a deterministic, sub-millisecond hasher for test environments (`cfg(test)` or test fixtures)
      without altering application authentication code.

## Boundaries & Non-Responsibilities

- **Not for Reversible Encryption**: Password hashing is strictly one-way and memory-hard. Reversible data encryption,
  tokens,
  and signed URLs belong in `crates/crypto`.
- **Not Coupled to User Entities or Database**: Operates purely on strings and configuration. Database persistence and
  ORM
  integration belong in `crates/auth` or user models.
- **No Fast Hashes for Passwords**: SHA-256, MD5, and fast digests must never be used as password hash drivers.

## Proposed Public Surface

- `trait Hasher`: Core password hashing contract.
- `struct HashManager`: Configuration-driven registry and facade.
- `struct Argon2Hasher`: Argon2id driver with configurable parameters.
- `struct BcryptHasher`: Bcrypt driver with configurable rounds.
- `struct FakeHasher`: Blazing-fast test double.
- `struct Argon2Config`, `struct BcryptConfig`: Driver configurations.
- `enum HashAlgorithm`: `Argon2id`, `Bcrypt`, `Fake`.
- `enum HashError`: Strongly typed errors for invalid PHC formats, parameter parsing failures, and verification errors.
