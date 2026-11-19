# `auth` Crate Context & Architecture

## Vision & Scope

The `auth` crate provides a unified, multi-guard authentication system for the Druto web framework. It manages user
identity resolution, credential verification, session-based logins, and stateless API token authentication.

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/auth` (multi-guard authenticators: session guard, access tokens guard, basic auth guard)
- **Laravel**: `Illuminate\Auth` (`AuthManager`, `SessionGuard`, `TokenGuard`, `Sanctum`)
- **Loco**: `loco::auth`

## Key Capabilities & Responsibilities

1. **Multi-Guard Architecture**:
    - **Session Guard**: State-backed authentication utilizing `crates/session` and `crates/cookie` with session
      fixation
      protection and remember-me tokens.
    - **Access Token Guard**: Cryptographically signed bearer token authentication (`Bearer <token>`) for APIs and SPAs.
    - **API Key Guard**: Header or query parameter API key authentication with constant-time verification.

2. **User Provider & `Authenticatable` Trait**:
    - Abstract user identity trait decoupled from underlying database ORMs or persistence layers.
    - Resolves users by unique identifier, email/username, or token digest.

3. **Pipeline Middlewares**:
    - `AuthenticateMiddleware`: Ensures request has valid identity, rejecting unauthenticated requests with
      `401 Unauthorized`.
    - `GuestMiddleware`: Ensures incoming request is from an unauthenticated visitor (redirecting logged-in users away
      from login pages).

4. **Security Hardening**:
    - Constant-time token and secret comparisons via `subtle::ConstantTimeEq`.
    - Integrates with `crates/hash` for password verification and rehashing.
