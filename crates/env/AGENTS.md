# `env` Agent Instructions

## Scope

This crate owns `.env` file reading/parsing, variable interpolation, and boot-time schema validation. It runs synchronously at application startup.

## Design Rules

1. **Security & Secret Redaction**:
   - Error messages and debug outputs must NEVER print secret values in cleartext. Report the variable name and the constraint that failed (e.g. `APP_KEY: must be at least 32 characters long`), never the actual secret value.
2. **Comprehensive Validation Aggregation**:
   - Do not stop validation on the first error. Collect all missing, unparseable, or invalid variables into a unified error collection so developers can fix all configuration issues in one pass.
3. **Cyclic Reference Detection**:
   - Prevent infinite loops during variable interpolation (e.g. `A=${B}`, `B=${A}`) by capping expansion depth or tracking resolution history.
4. **No Clippy Suppressions**:
   - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code or resolve the warning cleanly.
5. **No Leading Underscores**:
   - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
6. **Comments**:
   - Comment only the "why" when code alone cannot state the external constraint or intentional design decision. Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **Syntax Edge Cases**: Single quotes (literal), double quotes (escaped characters), unquoted strings, whitespace handling, comment handling, empty values.
- **Interpolation Tests**: Nested variables, default fallbacks (`${FOO:-bar}`), missing variable handling, cyclic recursion checks.
- **Type Coercion Tests**: Boolean variants (`true`, `false`, `1`, `0`, `yes`, `no`), integers, ports, enum value constraints.
- **Diagnostic Output**: Verify error output formatting when multiple variables fail validation.
