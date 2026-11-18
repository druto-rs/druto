# `druto` Agent Instructions

## Scope

This crate is the top-level facade. It re-exports underlying crates and exposes the primary application builder and prelude.

## Design Rules

1. **Clean Re-exports**:
   - Re-export modules under logical, idiomatic namespaces (`druto::routing`, `druto::middleware`, etc.).
   - Expose a carefully curated `druto::prelude` containing only the traits, macros, and types essential for everyday application code.
2. **Feature Flags**:
   - Map workspace crate dependencies behind sensible feature flags so minimal or API-only applications can strip unused subsystems.
3. **No Clippy Suppressions**:
   - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code or resolve the warning cleanly.
4. **No Leading Underscores**:
   - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
5. **Comments**:
   - Comment only the "why" when code alone cannot state the external constraint or intentional design decision. Document public surfaces with rustdoc (`///`, `//!`).
