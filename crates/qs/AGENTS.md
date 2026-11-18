# `qs` Agent Instructions

## Scope

This crate owns nested query string parsing and serialization using bracket syntax. It must maintain strict defensive
resource boundaries to prevent memory exhaustion and recursion attacks.

## Design Rules

1. **Defensive Resource Limits**:
    - Depth, parameter count, and sparse array limits must always be enforced. Never allocate unbounded memory based on
      user input.
2. **RFC 3986 Compliance**:
    - Percent-decoding must distinguish between literal `+` (converted to space in query strings) and encoded `%2B`
      (preserved as plus).
3. **Deterministic Output**:
    - `stringify` must produce deterministic, alphabetically sorted query strings to allow stable hashing and signature
      verification.
4. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`).
5. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
6. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).

## Testing Requirements

- **Deep Nesting**: Verify multi-level object parsing (`a[b][c][d]=val`) and serialization.
- **Array Parsing**: Test sequential bracket (`a[]=1&a[]=2`) and indexed bracket (`a[0]=1&a[1]=2`) parsing.
- **Depth Limit Enforcement**: Verify that exceeding `max_depth` fails gracefully with a typed error.
- **Sparse Array Protection**: Verify that high indices (e.g. `items[999999]=x`) are rejected rather than allocating
  huge arrays.
- **Percent Encoding Roundtrip**: Test special characters, spaces, Unicode, and plus signs through parse and stringify.
