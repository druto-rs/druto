# `env` Crate Context

## Purpose

`env` provides environment variable file parsing, variable expansion (interpolation), and compile-time/boot-time schema
validation for the Druto web framework.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/env` (`Env.rules()`)
- **Laravel**: `Dotenv` + config integration
- **Prior Rust art**: `dotenvy`, `envy`

Unlike simple `.env` loaders that inject raw untyped strings into `std::env`, `env` enforces that an application cannot
boot with missing, invalid, or malformed configuration.

## Core Responsibilities

1. **`.env` File Parsing**:
    - Parse key-value definitions with inline comments (`#`), single quotes, double quotes, and unquoted values.
    - Support multi-line quoted strings (both single and double quoted).
    - Support escape sequences in double-quoted strings (`\n`, `\r`, `\t`, `\\`, `\"`, `\$`).
    - Support optional `export ` prefix.
2. **Variable Expansion & Interpolation**:
    - Expand referenced variables using `${VAR}`, `$VAR`, and `${VAR:-default}` syntax.
    - Resolve file-based secrets via `file:<path>` protocol.
    - Detect and prevent infinite recursion or cyclical expansions using cycle tracking.
3. **Cascading File Priority**:
    - Load configuration cascading in AdonisJS-compatible priority order:
        1. `.env.{env}.local` (highest priority)
        2. `.env.local` (loaded unless in `test` environment)
        3. `.env.{env}`
        4. `.env` (base file)
    - Determine environment via `DRUTO_ENV` -> `APP_ENV` -> `"development"`.
    - Respect system environment variable precedence over file values by default (with opt-in system override flag).
4. **Schema Definition & Validation**:
    - Provide a fluent rule builder defining expected variables, their types, and constraints:
        - `string()` (with optional min/max length, regex, prefixes, suffixes, choices/enums)
        - `integer()` (with optional range)
        - `float()`
        - `boolean()` (parses `true`, `false`, `1`, `0`, `yes`, `no`)
        - `url()`
        - `port()` (valid 1-65535 range)
            - `duration()` (parses `ms`, `s`, `m`, `h`, `d` into `std::time::Duration`)
            - `bytes()` (parses `b`, `kb`, `mb`, `gb`, `tb` into `u64`)
        - `enum_values(&["local", "production", "test"])`
            - `list()` (parses comma-separated values into `Vec<T>`)
            - `secret()` (wraps values in `Secret<T>` to prevent accidental leak)
    - Support default values and optional vs required declarations.
5. **Secret Protection**:
    - Provide a `Secret<T>` wrapper masking raw values as `[REDACTED]` on `Debug`, `Display`, and `Serialize`.
    - Access to inner secret value requires explicit `.expose_secret()` or `.into_inner()`.
6. **Round-Trip File Editing**:
    - Provide `EnvEditor` for modifying `.env` files while preserving file layout, comments, and empty lines.
    - Support programmatic synchronization of missing keys into `.env.example` templates.
7. **Actionable Diagnostics**:
    - Aggregate all validation failures into a structured, human-readable error report listing all missing or invalid
      variables at once, rather than failing on the first error.

## Boundaries & Non-Responsibilities

- **No Remote Secrets**: Does not fetch secrets over the network (e.g. AWS Secrets Manager, HashiCorp Vault); that
  belongs to external provider adapters.
- **Explicit Process Mutation**: Prefers returning a validated, immutable typed configuration store (`Env`); mutating
  `std::env` requires explicit opt-in via `unsafe fn export_to_process()`.

## Target Public Surface

- `Env`: Immutable typed configuration store (`get::<T>`, `get_raw`, `contains`, `deserialize`, `export_to_process`,
  `rules()`).
- `EnvLoader`: Cascading file loader with environment awareness, custom base directory, and system environment override
  options.
- `EnvParser`: Low-level `.env` syntax parser with multiline, quote, escape, and inline comment support.
- `EnvEditor`: Comment-preserving round-trip `.env` file editor with `.env.example` key synchronization.
- `EnvValidator`: Schema validator aggregating rule checks across keys.
- `Rule<T>`: Fluent rule builder supporting string constraints, numerics, booleans, ports, URLs, durations, bytes,
  enums, lists, and secrets.
- `Secret<T>`: Secret wrapper redacting cleartext on `Debug`, `Display`, and `Serialize`.
- `EnvError`: Rich diagnostic error type with aggregated validation failures (`ValidationFailed`).
