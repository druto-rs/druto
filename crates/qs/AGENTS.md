# `qs` Agent Instructions

## Scope

This crate owns nested bracket-notation query string parsing and serialization. It provides both a dynamic tree
representation (`QsValue`) and high-level Serde integration (`from_str`, `to_string`). It must maintain strict defensive
resource boundaries to prevent memory exhaustion, stack overflow, and CPU hash-collision attacks.

## Design Rules

1. **Defensive Resource Limits**:
    - Limits for recursion depth, parameter count, and sparse array indexing must always be enforced.
    - Never allocate unbounded vectors or maps based on untrusted user input (e.g. `items[9999999]=val`).
    - Guard against stack overflow during deeply nested bracket parsing.
2. **RFC 3986 & Query String Encoding**:
    - Distinguish between literal `+` (percent-decoded to space `' '` in query strings) and encoded `%2B` (preserved as
      `'+'`).
    - `stringify` must percent-encode reserved characters accurately according to RFC 3986, respecting
      `encode_values_only` when requested.
3. **Deterministic Output**:
    - Serialization must sort map keys alphabetically by default to guarantee deterministic output across runs, ensuring
      cache friendliness and valid cryptographic HMAC signatures.
4. **Serde Coercion & Ergonomics**:
    - When deserializing query parameters into strongly typed Rust structs, coerce string primitives into target numeric
      types (`u8`..`u128`, `i8`..`i128`, `f32`/`f64`) and booleans (`"true"`/`"false"`).
5. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Restructure the code cleanly
      instead.
6. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
7. **Comments**:
    - Comment only the "why" when code alone cannot state an external constraint or intentional design decision.
      Document public surfaces thoroughly with rustdoc (`///`, `//!`).

## Testing Requirements

- **Deep Nesting**: Verify multi-level object parsing (`a[b][c][d]=val`) and serialization roundtrips.
- **Strict Depth & Non-Strict Depth**: Test that `strict_depth: true` errors on excess depth, while
  `strict_depth: false` preserves remainder brackets in key names.
- **Array Parsing**: Test sequential empty-bracket (`a[]=1&a[]=2`) and indexed bracket (`a[0]=1&a[1]=2`) parsing.
- **Array Limit & Sparse Arrays**: Test that high indices (e.g. `items[999999]=x`) are handled safely without unbounded
  allocations. Test `allow_sparse` compaction.
- **Dot Notation**: Test mixed dot and bracket notation (`a.b[c]=d`) when `allow_dots` is enabled.
- **Comma Separation**: Test comma-separated values (`a=1,2,3`) when `comma` is enabled.
- **Duplicate Key Handling**: Test `Duplicates::Combine`, `Duplicates::First`, and `Duplicates::Last`.
- **Stringify Array Formats**: Verify serialization output across `Indices`, `Brackets`, `Repeat`, and `Comma` array
  formats.
- **Serde Deserialization**: Test deserialization into Rust structs containing strings, numbers, booleans, optional
  fields, nested structs, and `Vec<T>`.
- **Percent-Encoding Roundtrip**: Test special characters, spaces, Unicode, and plus signs through parse and stringify.
