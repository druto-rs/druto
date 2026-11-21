# Druto Framework Workspace Context

## Vision

Druto is a full-stack, "batteries-included" web framework for Rust, built with the developer ergonomics, cohesion, and
convention-over-configuration found in **Laravel** and **AdonisJS**.

The defining architectural principle of Druto is **total stack ownership**. Rather than acting as a loose template
gluing unrelated third-party libraries together, Druto designs and implements its foundational components from the
ground up as deep, modular crates.

## Reference Blueprint & Equivalent Mapping

The framework is modeled on the layered design of AdonisJS (`@poppinss/*` and `@adonisjs/*`) and Laravel
(`illuminate/*`):

| Layer                        | Responsibility                                                    | Druto Crate       | AdonisJS Equivalent      | Laravel Equivalent                    |
|------------------------------|-------------------------------------------------------------------|-------------------|--------------------------|---------------------------------------|
| **Layer 0: Primitives**      | Path pattern matching, parameter extraction & HTTP method routing | `matcher`         | `@poppinss/matchit`      | `illuminate/routing` (trie)           |
| **Layer 0: Primitives**      | Async onion middleware pipeline                                   | `dhap` (ext)      | `@poppinss/middleware`   | `illuminate/pipeline`                 |
| **Layer 0: Primitives**      | URL pattern matching, parameter extraction & method routing       | `poth` (ext)      | `@adonisjs/http-server` (Route) | `illuminate/routing` (trie)   |
| **Layer 0: Primitives**      | String inflection, pluralization, casing                          | `borno` (ext)     | `@poppinss/string`       | `illuminate/support` (Str/Pluralizer) |
| **Layer 0: Primitives**      | Nested bracket-notation query string parser & serializer          | `khonj` (ext)     | `@poppinss/qs`           | `illuminate/http` (query parser)      |
| **Layer 0: Primitives**      | Typed asynchronous pub/sub event dispatcher & testing fakes       | `barta` (ext)     | `@adonisjs/events`       | `illuminate/events`                   |
| **Layer 0: Primitives**      | AEAD encryption, HMAC signing, key rotation, signed URLs          | `gopon` (ext)     | `@boringnode/encryption` | `illuminate/encryption`               |
| **Layer 0: Primitives**      | Password hashing drivers (Argon2id, Bcrypt), rehash detection     | `mishron` (ext)   | `@adonisjs/hash`         | `illuminate/hashing`                  |
| **Layer 1: Configuration**   | .env parsing & boot-time schema validation                        | `poribesh` (ext)  | `@adonisjs/env`          | `Dotenv` + `config`                   |
| **Layer 2: State & Storage** | RFC 6265 cookies, signed & encrypted cookie jars                  | `chirkut` (ext)   | `@adonisjs/http-server`  | `illuminate/cookie`                   |
| **Layer 2: State & Storage** | Multi-driver session management & flash data                      | `session`         | `@adonisjs/session`      | `illuminate/session`                  |
| **Layer 2: State & Storage** | Asynchronous Redis connection manager, pooling & pub/sub          | `redis`           | `@adonisjs/redis`        | `illuminate/redis`                    |
| **Layer 2: State & Storage** | Unified multi-driver cache, tiered L1/L2, stampede defense & tags | `kouta` (ext)     | `@adonisjs/cache`        | `illuminate/cache`                    |
| **Layer 3: HTTP**            | HTTP router, request, response, groups, context, URL generator    | `router`          | `@adonisjs/http-server`  | `illuminate/routing`, `http`          |
| **Layer 3: HTTP**            | Cross-Origin Resource Sharing (CORS) preflight & response headers | `cors`            | `@adonisjs/cors`         | `fruitcake/laravel-cors`              |
| **Layer 3: HTTP**            | Web security hardening (CSRF, CSP, HSTS, FrameGuard, NoSniff)     | `shield`          | `@adonisjs/shield`       | `VerifyCsrfToken` + Security Headers  |
| **Layer 3: HTTP**            | Rate limiting, token bucket, sliding window & HTTP headers        | `limiter`         | `@adonisjs/limiter`      | `illuminate/cache` (RateLimiter)      |
| **Layer 3: HTTP**            | Request schema definition, validation rules, sanitization, errors | `jachai` (ext)    | `vinejs`                 | `illuminate/validation`               |
| **Layer 3: HTTP**            | Streaming multipart parser, file upload handling, size limits     | `bojha` (ext) / `bodyparser` | `@adonisjs/bodyparser`   | `illuminate/http` (UploadedFile)      |
| **Layer 3: HTTP**            | High-performance static asset file serving, range requests, ETags | `staticfiles`     | `@adonisjs/static`       | `public/` / `ServeFile`               |
| **Layer 3: HTTP**            | Authentication guards, session/token authenticators, identity     | `auth`            | `@adonisjs/auth`         | `illuminate/auth`                     |
| **Layer 4: Data**            | Query builder, migrations, ORM                                    | `db` *(planned)*  | `@adonisjs/lucid`        | `illuminate/database` (Eloquent)      |
| **Layer 5: Developer Tools** | CLI command suite, generators                                     | `druto::cli` / `karigor` (ext)    | `@adonisjs/ace`          | `illuminate/console` (Artisan)        |
| **Layer 6: Facade**          | Framework umbrella & prelude                                      | `druto`           | `@adonisjs/core`         | `laravel/framework`                   |

## Facade Architecture

`druto` is structured as a single-package umbrella facade crate that aggregates and provides unified entry points, developer ergonomics, and preconfigured defaults across the modular framework ecosystem.

### Modular Ecosystem Repositories (`druto-rs`)

- **Router**: [`druto-rs/router`](https://github.com/druto-rs/router) — HTTP routing, request/response, groups, context, URL generation
- **Bodyparser**: [`druto-rs/bodyparser`](https://github.com/druto-rs/bodyparser) — Multipart decoding, file uploads, size quotas
- **CORS**: [`druto-rs/cors`](https://github.com/druto-rs/cors) — Cross-Origin Resource Sharing preflight and headers
- **Limiter**: [`druto-rs/limiter`](https://github.com/druto-rs/limiter) — Rate limiting, token bucket, sliding window, HTTP headers
- **Shield**: [`druto-rs/shield`](https://github.com/druto-rs/shield) — Web application security suite (CSRF, CSP, FrameGuard, HSTS)
- **Session**: [`druto-rs/session`](https://github.com/druto-rs/session) — Multi-driver HTTP session management, flash data, fixation defense
- **Redis**: [`druto-rs/redis`](https://github.com/druto-rs/redis) — Asynchronous Redis client manager, connection pooling, and pub/sub
- **Ignitor**: [`druto-rs/ignitor`](https://github.com/druto-rs/ignitor) — Application bootstrapper, runtime lifecycle manager, and server orchestrator
- **Karigor (External Primitives)**: [`Hitarashi/karigor`](https://github.com/Hitarashi/karigor) — Framework-agnostic CLI toolkit, kernel, command trait, UI and template engine
