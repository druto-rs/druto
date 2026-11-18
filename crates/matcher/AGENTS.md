# `matcher` Agent Instructions

## Scope

This crate owns URL pattern parsing, route tree indexing, parameter extraction, and HTTP method routing. It has no
network, async, or transport dependencies.

## Design Rules

1. **Zero Allocation in the Fast Path**:
    - Path lookups must avoid allocating heap memory wherever possible.
    - Return parameter slices `&'a str` referencing the input path string rather than owned `String` instances.
    - Store parameters in stack-backed buffers for up to 16 parameters.
2. **Deterministic Precedence**:
    - Order of route registration must not produce nondeterministic matches.
    - Ambiguities must follow strict priority: Static > Regex Constrained > Built-in Constrained > Dynamic Param >
      Wildcard.
3. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code or resolve the
      warning cleanly.
4. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
5. **Comments & Documentation**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).
    - Do not reference external frameworks in rustdoc comments.

## Testing Requirements

- **Unit Tests**: Full coverage of edge cases (trailing slashes, duplicate slashes, URL-encoded segments, unicode
  paths).
- **Precedence Tests**: Explicit tests verifying that static routes win over parameters, and parameters win over
  wildcards regardless of insertion order.
- **Property-Based Testing**: Use `proptest` to test arbitrary path structures against tree invariants.
