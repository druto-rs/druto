# `druto` Agent Instructions

## Scope

This crate is the top-level facade. It re-exports underlying crates and exposes the primary application builder and prelude.

## Design Rules

1. **Clean Re-exports**:
   - Re-export modules under logical, idiomatic namespaces (`druto::routing`, `druto::middleware`, etc.).
   - Expose a carefully curated `druto::prelude` containing only the traits, macros, and types essential for everyday application code.
2. **Feature Flags**:
   - Map workspace crate dependencies behind sensible feature flags so minimal or API-only applications can strip unused subsystems.
