# Comments

Comment the why, and only when the code cannot state it: a constraint from
outside the file, a deliberate omission, or a decision whose alternative a
reader would otherwise pick.

If a competent reader could infer it from the code, the code already says it.
Rustdoc (`///`, `//!`) is always fine — it documents the public surface.

# Code Style and Lints

- Follow [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.md).
- Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`) at all. Resolve the warning or
  restructure the code instead.
- Avoid using leading underscores (`_`) on identifiers, variables, parameters, or struct fields.
- Code formatting must be verified via `cargo +nightly fmt --check`.
- Always check local repo `.md` files whether ignored or not.

# Framework-Agnostic Documentation & Codebase

- Do not mention any other web frameworks (e.g., AdonisJS, Laravel, Express, Axum, Actix, Rails, Django) in rustdocs,
  codebase
  comments, identifiers, or error messages.
- Reference only RFCs, W3C standards, official specifications, and Druto types.

# Skills

Activate and adhere to the following specialized skills when applicable:

- **`rust-best-practices`**: Use when writing, reviewing, or refactoring Rust code, designing types and error handling
  with `Result`, choosing ownership/borrowing patterns, or optimizing performance.
- **`rust-testing`**: Use when writing unit, integration, async, or property-based tests, setting up mocks, or measuring
  test coverage.
- **`codebase-design`**: Use when designing module interfaces, defining seams and boundaries, or improving testability
  and code structure.
- **`enterprise-ux`**: Use when designing, reviewing, or improving UI layouts, workflow ergonomics, data density, or
  navigation patterns.
- **`diagnosing-bugs`**: Use when investigating build failures, runtime errors, panics, or performance regressions.

# Crate-Specific Context & Instructions

When working within a specific crate, consult its local `CONTEXT.md` and `AGENTS.md`:

- **Root Context**: [`CONTEXT.md`](CONTEXT.md)
- **`crates/auth`**: [`CONTEXT.md`](crates/auth/CONTEXT.md) | [`AGENTS.md`](crates/auth/AGENTS.md)
- **`crates/bodyparser`**: [`CONTEXT.md`](crates/bodyparser/CONTEXT.md) | [`AGENTS.md`](crates/bodyparser/AGENTS.md)
- **`crates/cors`**: [`CONTEXT.md`](crates/cors/CONTEXT.md) | [`AGENTS.md`](crates/cors/AGENTS.md)
- **`crates/druto`**: [`CONTEXT.md`](crates/druto/CONTEXT.md) | [`AGENTS.md`](crates/druto/AGENTS.md)
- **`crates/limiter`**: [`CONTEXT.md`](crates/limiter/CONTEXT.md) | [`AGENTS.md`](crates/limiter/AGENTS.md)
- **`crates/redis`**: [`CONTEXT.md`](crates/redis/CONTEXT.md) | [`AGENTS.md`](crates/redis/AGENTS.md)
- **`crates/router`**: [`CONTEXT.md`](crates/router/CONTEXT.md) | [`AGENTS.md`](crates/router/AGENTS.md)
- **`crates/session`**: [`CONTEXT.md`](crates/session/CONTEXT.md) | [`AGENTS.md`](crates/session/AGENTS.md)
- **`crates/shield`**: [`CONTEXT.md`](crates/shield/CONTEXT.md) | [`AGENTS.md`](crates/shield/AGENTS.md)
- **`crates/staticfiles`**: [`CONTEXT.md`](crates/staticfiles/CONTEXT.md) | [`AGENTS.md`](crates/staticfiles/AGENTS.md)
