# `qs` Crate Context

## Purpose

`qs` provides nested, bracket-notation query string parsing and serialization for the Druto framework. It allows
complex structured parameters (nested objects, lists, and associative arrays) sent by modern web frontends to be
parsed into structured types with strict limits preventing algorithmic complexity and stack exhaustion attacks.

## Reference Architecture

This crate is the Rust equivalent of:

- **Poppinss**: `@poppinss/qs` (`lib/parse.js`, `lib/stringify.js`)
- **Laravel**: `illuminate/http` (Nested query parameter extraction)
- **Node.js**: `qs` library (RFC 3986 query parameter parser)

In standard Rust, crates such as `serde_urlencoded` only handle flat `key=value` pairs and fail when confronted with
bracket notation (`filter[author][id]=42&sort[]=created_at`). `qs` fills this critical framework gap.

## Core Responsibilities

1. **Nested Bracket Syntax Parsing**:
    - Parses deep associative objects: `user[profile][name]=John` $\to$ `{"user": {"profile": {"name": "John"}}}`.
    - Parses sequential and explicit arrays: `ids[]=1&ids[]=2` and `ids[0]=1&ids[1]=2` $\to$ `{"ids": ["1", "2"]}`.
    - Handles mixed dot and bracket notation where appropriate.
2. **Defensive Resource Limits (DoS & HashDoS Prevention)**:
    - **Depth Limit** (default: 5): Rejects or truncates recursive nesting beyond safe limits to prevent stack overflow
      attacks.
    - **Parameter Count Limit** (default: 1000): Enforces strict maximum key count to protect against CPU exhaustion
      attacks.
    - **Array Allocation Limit** (default: 100): Prevents memory exhaustion attacks via sparse indices (e.g.
      `items[1000000000]=val`).
3. **Structured Serialization (`stringify`)**:
    - Serializes maps, arrays, and scalar values into canonical RFC 3986 query strings with bracket notation.
4. **RFC 3986 Percent-Decoding & Query Space Handling**:
    - Correctly handles `+` conversion to space in query strings while preserving encoded `%2B` plus signs.

## Boundaries & Non-Responsibilities

- **Not an HTTP Request Object**: Operates on raw query strings and returns structured dynamic values (`QsValue`) or
  deserializes into types.
- **Not a Routing Parameter Matcher**: Route path parameter matching belongs in `crates/matcher`.

## Proposed Public Surface

- `enum QsValue`: Represents dynamic query data (`Scalar(String)`, `Array(Vec<QsValue>)`,
  `Object(BTreeMap<String, QsValue>)`).
- `struct QsOptions`: Configurable limits (`max_depth`, `parameter_limit`, `array_limit`).
- `fn parse(query: &str, options: &QsOptions) -> Result<QsValue, QsError>`.
- `fn stringify(value: &QsValue, options: &QsOptions) -> String`.
- `enum QsError`: Typed errors for depth limit exceeded, parameter limit exceeded, and malformed syntax.
