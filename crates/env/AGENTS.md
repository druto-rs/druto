# `env` Agent Instructions

## Scope

This crate owns `.env` file reading/parsing, variable interpolation, and boot-time schema validation. It runs
synchronously at application startup.

## Design Rules

1. **Security & Secret Redaction**:
    - Error messages and debug outputs must NEVER print secret values in cleartext. Report the variable name and the
      constraint that failed (e.g. `APP_KEY: must be at least 32 characters long`), never the actual secret value.
2. **Comprehensive Validation Aggregation**:
    - Do not stop validation on the first error. Collect all missing, unparseable, or invalid variables into a unified
      error collection so developers can fix all configuration issues in one pass.
3. **Cyclic Reference Detection**:
    - Prevent infinite loops during variable interpolation (e.g. `A=${B}`, `B=${A}`) by capping expansion depth or
      tracking resolution history.

## Testing Requirements

- **Syntax Edge Cases**: Single quotes (literal), double quotes (escaped characters), unquoted strings, whitespace
  handling, comment handling, empty values, multiline strings.
- **Interpolation Tests**: Nested variables, default fallbacks (`${FOO:-bar}`), missing variable handling, cyclic
  recursion checks, and `file:<path>` secret resolution.
- **Cascading & Overrides**: File resolution hierarchy (`.env.{env}.local` down to `.env`), test environment exclusion
  of `.env.local`, and system environment overrides.
- **Type Coercion Tests**: Boolean variants (`true`, `false`, `1`, `0`, `yes`, `no`), integers, floats, ports, URLs,
  durations, byte sizes, enum values, lists, and secrets.
- **Secret Redaction**: Verification that `Secret<T>` obscures contents on `Debug`, `Display`, and `Serialize`.
- **Editor Round-Trip**: Verifying comments, blank lines, ordering preservation, and `.env.example` sync.
- **Diagnostic Output**: Verify error output formatting when multiple variables fail validation.
