# `druto` Crate Context

## Purpose

`druto` is the top-level umbrella facade for the Druto web framework. It re-exports and integrates the framework's
modular crates into a cohesive developer experience, delivering the complete "batteries-included" web framework vision
inspired by Laravel and AdonisJS.

## Architectural Philosophy

- **Total Stack Ownership**: Unlike meta-frameworks that glue together third-party routers and ORMs, Druto builds and
  owns its stack layer-by-layer.
- **Deep Modules**: High leverage behind intuitive interfaces.
- **Progressive Disclosure of Complexity**: Simple and ergonomic out of the box, yet deeply configurable via traits and
  providers.

## Workspace Ecosystem

`druto` unifies the workspace crates:

- `barta`: In-process pub/sub event dispatcher, wildcard topic routing, and testing fakes.
- `borno`: Grammatical inflections, casing transformations, and string conventions.
- `chirkut`: RFC 6265bis cookies, signed and encrypted cookie jars.
- `gopon`: Authenticated encryption, HMAC signing, key rotation, and signed URLs.
- `jachai`: Universal declarative schema definition, validation, sanitization, and structured error reporting.
- `khonj`: Nested bracket-notation query string parsing and serialization.
- `kouta`: Unified multi-driver caching engine, multi-tier stores, and stampede protection.
- `mishron`: Password hashing drivers (Argon2id, Bcrypt) and automatic rehash detection.
- `dhap`: Asynchronous middleware onion execution.
- `poribesh`: Environment variable loading, expansion, and boot-time schema validation.
- `poth`: URL pattern matching, parameter extraction, and HTTP method routing.
- `router`: HTTP routing, request/response models, middleware pipeline, and reverse URL generation.
- `session`: Multi-driver HTTP session management, flash data, and session fixation defense.
- *(Future)* `auth`, `validation`, `db`, `cli`.

## Responsibilities of `druto`

- Provide the unified `druto::prelude::*` for application authors.
- Serve as the developer-facing documentation entry point.
- Coordinate framework startup, service provider registration, and application lifecycle.
