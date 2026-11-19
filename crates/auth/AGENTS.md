# `auth` Agent Instructions

## Scope

This crate owns authentication guards, session and token authenticators, user identity resolution, and authentication
middlewares.

## Design Rules

1. **Defense-in-Depth & Constant-Time Operations**:
    - All credential and token verification checks must execute in constant time (`subtle::ConstantTimeEq`) to eliminate
      timing side-channels.
    - Password hashing and verification must delegate strictly to `crates/hash`.

2. **Session Fixation Prevention**:
    - On login and logout, the active session ID must be rotated / regenerated to prevent fixation attacks.
