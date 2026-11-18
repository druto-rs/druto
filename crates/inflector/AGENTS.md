# `inflector` Agent Instructions

## Scope

This crate owns string transformations, grammatical pluralization/singularization, and casing conventions for Druto. It is pure computation with zero network, filesystem, or async dependencies.

## Design Rules

1. **Deterministic & Idiomatic**:
   - Transformations must be deterministic and handle unicode boundary slicing safely without panicking.
2. **Extensive Grammar Rule Tables**:
   - Maintain comprehensive regular and irregular rule tables for English inflection.
   - Support case preservation (e.g. `User` pluralizes to `Users`, `user` to `users`, `USER` to `USERS`).
3. **Allocation Awareness**:
   - Provide zero-allocation checks (e.g., `is_singular`, `is_plural`) where possible.
   - For casing conversions where input is already in the target casing, consider `Cow<str>` to avoid unnecessary allocations.
4. **No Clippy Suppressions**:
   - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code or resolve the warning cleanly.
5. **No Leading Underscores**:
   - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
6. **Comments**:
   - Comment only the "why" when code alone cannot state the external constraint or intentional design decision. Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **Pluralization / Singularization Round-Tripping**:
  - Test extensive lists of regular and irregular nouns (`person` $\to$ `people` $\to$ `person`, `cactus` $\to$ `cacti` $\to$ `cactus`).
  - Test uncountable words (`money`, `rice`, `information`, `equipment`).
- **Casing Edge Cases**:
  - Strings with consecutive uppercase letters (acronyms like `HTTPResponse` $\to$ `http_response` vs `httpResponse`).
  - Leading, trailing, and duplicate separators (`__hello__world__` $\to$ `hello_world`).
  - Unicode characters and accents in slugification.
