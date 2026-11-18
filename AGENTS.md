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
- Always check local repo .md files whether ignored or not.

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
