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

| Layer                         | Responsibility                                                    | Druto Crate                   | AdonisJS Equivalent         | Laravel Equivalent                    |
|-------------------------------|-------------------------------------------------------------------|-------------------------------|-----------------------------|---------------------------------------|
| **Layer 0: Primitives**       | Path pattern matching, parameter extraction & HTTP method routing | `matcher`                     | `@poppinss/matchit`         | `illuminate/routing` (trie)           |
| **Layer 0: Primitives**       | Async onion middleware pipeline                                   | `pipeline`                    | `@poppinss/middleware`      | `illuminate/pipeline`                 |
| **Layer 0: Primitives**       | String inflection, pluralization, casing                          | `inflector`                   | `@poppinss/string`          | `illuminate/support` (Str/Pluralizer) |
| **Layer 1: Configuration**    | .env parsing & boot-time schema validation                        | `env`                         | `@adonisjs/env`             | `Dotenv` + `config`                   |
| **Layer 2: HTTP**             | HTTP server, router, request, response, errors                    | `router` / `http` *(planned)* | `@adonisjs/http-server`     | `illuminate/routing`, `http`          |
| **Layer 3: Security & State** | Sessions, cookies, CSRF, auth guards                              | `session`, `auth` *(planned)* | `@adonisjs/session`, `auth` | `illuminate/session`, `auth`          |
| **Layer 4: Data**             | Query builder, migrations, ORM                                    | `db` *(planned)*              | `@adonisjs/lucid`           | `illuminate/database` (Eloquent)      |
| **Layer 5: Developer Tools**  | CLI command suite, generators                                     | `cli` *(planned)*             | `@adonisjs/ace`             | `illuminate/console` (Artisan)        |
| **Layer 6: Facade**           | Framework umbrella & prelude                                      | `druto`                       | `@adonisjs/core`            | `laravel/framework`                   |

## Workspace Structure

```
crates/
├── druto/       # Framework umbrella facade
├── env/         # Environment variable loading, expansion, and schema validation
├── inflector/   # String inflections, casing transformations, and pluralization
├── matcher/     # Route pattern matching and parameter extraction
└── pipeline/    # Asynchronous middleware pipeline and execution runner
```

Each crate maintains its own `CONTEXT.md` (architecture and boundaries) and `AGENTS.md` (agent rules and constraints).
