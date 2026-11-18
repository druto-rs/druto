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

| Layer                        | Responsibility                                                    | Druto Crate                   | AdonisJS Equivalent      | Laravel Equivalent                    |
|------------------------------|-------------------------------------------------------------------|-------------------------------|--------------------------|---------------------------------------|
| **Layer 0: Primitives**      | Path pattern matching, parameter extraction & HTTP method routing | `matcher`                     | `@poppinss/matchit`      | `illuminate/routing` (trie)           |
| **Layer 0: Primitives**      | Async onion middleware pipeline                                   | `pipeline`                    | `@poppinss/middleware`   | `illuminate/pipeline`                 |
| **Layer 0: Primitives**      | String inflection, pluralization, casing                          | `inflector`                   | `@poppinss/string`       | `illuminate/support` (Str/Pluralizer) |
| **Layer 0: Primitives**      | Nested bracket-notation query string parser & serializer          | `qs`                          | `@poppinss/qs`           | `illuminate/http` (query parser)      |
| **Layer 0: Primitives**      | Typed asynchronous pub/sub event dispatcher & testing fakes       | `events`                      | `@adonisjs/events`       | `illuminate/events`                   |
| **Layer 0: Primitives**      | AEAD encryption, HMAC signing, key rotation, signed URLs          | `crypto`                      | `@boringnode/encryption` | `illuminate/encryption`               |
| **Layer 0: Primitives**      | Password hashing drivers (Argon2id, Bcrypt), rehash detection     | `hash`                        | `@adonisjs/hash`         | `illuminate/hashing`                  |
| **Layer 1: Configuration**   | .env parsing & boot-time schema validation                        | `env`                         | `@adonisjs/env`          | `Dotenv` + `config`                   |
| **Layer 2: State & Storage** | RFC 6265 cookies, signed & encrypted cookie jars                  | `cookie`                      | `@adonisjs/http-server`  | `illuminate/cookie`                   |
| **Layer 2: State & Storage** | Multi-driver session management & flash data                      | `session`                     | `@adonisjs/session`      | `illuminate/session`                  |
| **Layer 3: HTTP**            | HTTP server, router, request, response, errors                    | `router` / `http` *(planned)* | `@adonisjs/http-server`  | `illuminate/routing`, `http`          |
| **Layer 4: Data**            | Query builder, migrations, ORM                                    | `db` *(planned)*              | `@adonisjs/lucid`        | `illuminate/database` (Eloquent)      |
| **Layer 5: Developer Tools** | CLI command suite, generators                                     | `cli` *(planned)*             | `@adonisjs/ace`          | `illuminate/console` (Artisan)        |
| **Layer 6: Facade**          | Framework umbrella & prelude                                      | `druto`                       | `@adonisjs/core`         | `laravel/framework`                   |

## Workspace Structure

```
crates/
├── cookie/      # RFC 6265 cookies, plain, signed, and encrypted cookie jars
├── crypto/      # Authenticated encryption, HMAC signing, key rotation, signed URLs
├── druto/       # Framework umbrella facade
├── env/         # Environment variable loading, expansion, and schema validation
├── events/      # Strongly typed asynchronous event dispatcher & testing fakes
├── hash/        # Password hashing drivers, automatic rehash detection, verification
├── inflector/   # String inflections, casing transformations, and pluralization
├── matcher/     # Route pattern matching and parameter extraction
├── pipeline/    # Asynchronous middleware pipeline and execution runner
├── qs/          # Nested bracket-notation query string parser & serializer
└── session/     # Multi-driver HTTP session management, flash data, fixation defense
```

Each crate maintains its own `CONTEXT.md` (architecture and boundaries) and `AGENTS.md` (agent rules and constraints).
