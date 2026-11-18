# `env` Crate Context

## Purpose

`env` provides environment variable file parsing, variable expansion (interpolation), and compile-time/boot-time schema validation for the Druto web framework.

## Reference Architecture

This crate is the Rust equivalent of:
- **AdonisJS**: `@adonisjs/env` (`Env.rules()`)
- **Laravel**: `Dotenv` + config integration
- **Prior Rust art**: `dotenvy`, `envy`

Unlike simple `.env` loaders that inject raw untyped strings into `std::env`, `env` enforces that an application cannot boot with missing, invalid, or malformed configuration.

## Core Responsibilities

1. **`.env` File Parsing**:
   - Parse key-value definitions with comments (`#`), single quotes, double quotes, and unquoted values.
   - Support multi-line quoted strings.
   - Support optional `export ` prefix.
2. **Variable Expansion & Interpolation**:
   - Expand referenced variables using `${VAR}` and `${VAR:-default}` syntax.
   - Detect and prevent infinite recursion or cyclical expansions.
3. **Schema Definition & Validation**:
   - Provide a fluent rule builder defining expected variables, their types, and constraints:
     - `string()` (with optional min/max length, regex, choices/enums)
     - `integer()` (with optional range)
     - `float()`
     - `boolean()` (parses `true`, `false`, `1`, `0`, `yes`, `no`)
     - `url()`
     - `port()` (valid 1-65535 range)
     - `enum_values(&["local", "production", "test"])`
   - Support default values and optional vs required declarations.
4. **Actionable Diagnostics**:
   - Aggregate all validation failures into a structured, human-readable error report listing all missing or invalid variables at once, rather than failing on the first error.

## Boundaries & Non-Responsibilities

- **No Remote Secrets**: Does not fetch secrets over the network (e.g. AWS Secrets Manager, HashiCorp Vault); that belongs to external provider adapters.
- **No Global Mutation**: Prefers returning a validated, immutable typed configuration store rather than mutating global process state.

## Target Public Surface

- `Env`: Main entry point for loading and validating environment sources.
- `EnvValidator`: Schema builder for environment rules.
- `EnvError`: Rich error type containing structured diagnostic messages for validation failures.
- `Rule<T>`: Schema field validator.
