# `druto` Crate Context

## Purpose

`druto` is the top-level umbrella facade for the Druto web framework. It re-exports and integrates the framework's modular crates into a cohesive developer experience, delivering the complete "batteries-included" web framework vision inspired by Laravel and AdonisJS.

## Architectural Philosophy

- **Total Stack Ownership**: Unlike meta-frameworks that glue together third-party routers and ORMs, Druto builds and owns its stack layer-by-layer.
- **Deep Modules**: High leverage behind intuitive interfaces.
- **Progressive Disclosure of Complexity**: Simple and ergonomic out of the box, yet deeply configurable via traits and providers.

## Workspace Ecosystem

`druto` unifies the workspace crates:
- `matcher`: Fast URL pattern matching and parameter extraction.
- `pipeline`: Asynchronous middleware onion execution.
- `env`: Environment variable loading, expansion, and boot-time schema validation.
- `inflector`: Grammatical inflections, casing transformations, and string conventions.
- *(Future)* `router`, `http`, `session`, `auth`, `db`, `cli`.

## Responsibilities of `druto`

- Provide the unified `druto::prelude::*` for application authors.
- Serve as the developer-facing documentation entry point.
- Coordinate framework startup, service provider registration, and application lifecycle.
