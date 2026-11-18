# `qs` Crate Context

## Purpose

`qs` provides nested, bracket-notation query string parsing and serialization for the Druto framework. It allows
complex structured parameters (nested objects, lists, and associative arrays) sent by modern web frontends and API
clients to be parsed into structured AST values (`QsValue`) or deserialized directly into strongly typed Rust structs
via Serde, with strict defensive limits preventing algorithmic complexity, memory exhaustion, and stack overflow
attacks.

## Reference Architecture

This crate is the Rust equivalent of:

- **Poppinss**: `@poppinss/qs` (`lib/parse.js`, `lib/stringify.js`, `lib/utils.js`)
- **AdonisJS**: `@adonisjs/http-server` (`src/qs.ts`, `src/types/qs.ts`)
- **Laravel**: `illuminate/http` (Nested query parameter extraction and `ParameterBag`)
- **Node.js**: `qs` library (RFC 3986 query parameter parser)

### Comparative Analysis with Existing Rust Libraries

- **`serde_urlencoded`**:
    - Handles only flat `key=value` pairs.
    - Fails completely on bracket notation (`filter[user][id]=1&items[]=2`).
    - Suffers from known deserialization issues with numeric string values inside extractors (as documented in
      `loco-rs`).
- **`serde_qs`**:
    - Offers bracket syntax deserialization but lacks fine-grained dynamic AST manipulation (`QsValue`).
    - Lacks configurable serialization formats (`indices`, `brackets`, `repeat`, `comma`).
    - Missing the explicit DoS protections, flexible duplicate resolution policies, and option parity found in
      `@poppinss/qs`.
- **Druto `qs`**:
    - Bridges this gap by providing both a rich dynamic AST (`QsValue`) and full Serde `Serializer`/`Deserializer`
      support.
    - Adheres strictly to `@poppinss/qs` and `@adonisjs/http-server` specifications with defensive resource limits.

## Core Responsibilities

1. **Nested Bracket & Dot Syntax Parsing**:
    - **Associative Objects**: Parses deep structures such as `user[profile][name]=John` into
      `{"user": {"profile": {"name": "John"}}}`.
    - **Indexed Arrays**: Parses explicit zero-based indices `ids[0]=1&ids[1]=2` into `{"ids": ["1", "2"]}`.
    - **Sequential Arrays**: Parses empty-bracket syntax `ids[]=1&ids[]=2` into `{"ids": ["1", "2"]}`.
    - **Dot Notation**: When `allow_dots` is enabled, transforms `user.name=John` or `user.address[city]=Paris` into
      nested objects.
    - **Comma-Separated Values**: When `comma` is enabled, splits unbracketed comma-delimited strings (`ids=1,2,3`) into
      array elements.
2. **Defensive Resource Limits (DoS & HashDoS Prevention)**:
    - **Depth Limit** (`depth`, default: 5): Limits recursion depth. When `strict_depth` is `false`, excess brackets
      remain unparsed within the key name; when `strict_depth` is `true`, an error is returned.
    - **Parameter Count Limit** (`parameter_limit`, default: 1000): Enforces a hard ceiling on parsed key-value pairs to
      protect against hash-collision and CPU-exhaustion attacks.
    - **Array Index & Allocation Limit** (`array_limit`, default: 20, matching AdonisJS): Prevents memory exhaustion
      attacks from sparse indices (e.g., `items[1000000]=val`). Indices above the limit either convert to map keys or
      trigger a rejection.
    - **Sparse Array Compaction** (`allow_sparse`, default: `false`): Compacts sparse arrays into contiguous vectors,
      avoiding empty element allocations.
3. **Structured Serialization (`stringify`)**:
    - Supports multiple array serialization formats:
        - `Indices`: `ids[0]=1&ids[1]=2` (AdonisJS default)
        - `Brackets`: `ids[]=1&ids[]=2`
        - `Repeat`: `ids=1&ids=2`
        - `Comma`: `ids=1,2`
    - **Deterministic Key Sorting**: Serializes map keys in deterministic alphabetical order to guarantee stable cache
      keys and reliable HMAC signatures for signed URLs.
    - **RFC 3986 Percent-Encoding**: Encodes query components according to RFC 3986, with an `encode_values_only` option
      that keeps bracket notation legible on the wire.
    - **Null and Empty Value Handling**: Configurable `skip_nulls` and `strict_null_handling` for query strings.
4. **RFC 3986 Percent-Decoding & Query Space Handling**:
    - Correctly converts literal `+` to spaces while preserving percent-encoded `%2B` as plus characters.
    - Optionally strips leading query prefixes (`?` via `ignore_query_prefix`).
    - Configurable duplicate key handling (`duplicates`): `Combine` (turn into array), `First` (keep initial value), or
      `Last` (overwrite with last value).
5. **First-Class Serde Integration**:
    - Provides `from_str` and `from_str_with_options` for deserializing directly into strongly typed Rust structs.
    - Provides `to_string` and `to_string_with_options` for serializing structs and collections into query strings.
    - Supports type coercion for string-encoded primitives (integers, floats, booleans) to eliminate friction with web
      forms and query parameters.

## Boundaries & Non-Responsibilities

- **Not an HTTP Request Object**: Operates on raw query strings and returns structured dynamic values (`QsValue`) or
  deserializes into types.
- **Not a Routing Parameter Matcher**: Route path parameter matching belongs in `crates/matcher`.

## Implemented Public Surface

### Data Model

- `enum QsValue`:
    - `Null`
    - `Scalar(String)`
    - `Array(Vec<QsValue>)`
    - `Object(BTreeMap<String, QsValue>)`

### Configuration Options

- `struct ParseOptions`:
    - `depth: usize` (default: 5)
    - `strict_depth: bool` (default: false)
    - `parameter_limit: usize` (default: 1000)
    - `array_limit: usize` (default: 20)
    - `allow_sparse: bool` (default: false)
    - `allow_dots: bool` (default: false)
    - `comma: bool` (default: false)
    - `duplicates: Duplicates` (default: `Duplicates::Combine`)
    - `ignore_query_prefix: bool` (default: true)
    - `strict_null_handling: bool` (default: false)
    - `delimiter: u8` (default: `b'&'`)
    - `decode_dot_in_keys: bool` (default: false)
- `enum Duplicates`:
    - `Combine`
    - `First`
    - `Last`
- `struct StringifyOptions`:
    - `array_format: ArrayFormat` (default: `ArrayFormat::Indices`)
    - `encode: bool` (default: true)
    - `encode_values_only: bool` (default: false)
    - `add_query_prefix: bool` (default: false)
    - `skip_nulls: bool` (default: false)
    - `strict_null_handling: bool` (default: false)
    - `sort: bool` (default: true)
    - `delimiter: u8` (default: `b'&'`)
    - `allow_dots: bool` (default: false)
- `enum ArrayFormat`:
    - `Indices`
    - `Brackets`
    - `Repeat`
    - `Comma`

### Error Handling

- `enum QsError`:
    - `DepthLimitExceeded(usize)`
    - `ParameterLimitExceeded(usize)`
    - `ArrayLimitExceeded(usize)`
    - `InvalidSyntax(String)`
    - `Custom(String)`

### Core Functions

- `fn parse(query: &str) -> Result<QsValue, QsError>`
- `fn parse_with_options(query: &str, options: &ParseOptions) -> Result<QsValue, QsError>`
- `fn stringify(value: &QsValue) -> String`
- `fn stringify_with_options(value: &QsValue, options: &StringifyOptions) -> String`
- `fn from_str<T: serde::de::DeserializeOwned>(query: &str) -> Result<T, QsError>`
- `fn from_str_with_options<T: serde::de::DeserializeOwned>(query: &str, options: &ParseOptions) -> Result<T, QsError>`
- `fn to_string<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, QsError>`
-

`fn to_string_with_options<T: serde::Serialize + ?Sized>(value: &T, options: &StringifyOptions) -> Result<String, QsError>`

- `fn to_value<T: serde::Serialize + ?Sized>(value: &T) -> Result<QsValue, QsError>`

## Test Coverage & Verification Metrics

- **Total Test Suite**: 73 tests (61 integration/proptest cases in `tests/qs_spec.rs`, 12 unit tests in
  `crates/qs/src/`).
- **Line Coverage**: **98.49%** (1435 of 1457 lines executed) via `cargo llvm-cov`.
    - `encoding.rs`: **100.00%**
    - `error.rs`: **100.00%**
    - `lib.rs`: **100.00%**
    - `options.rs`: **100.00%**
    - `stringify.rs`: **100.00%**
    - `value.rs`: **100.00%**
    - `ser.rs`: **99.37%**
    - `parser.rs`: **98.25%**
    - `de.rs`: **97.13%**
- **Function Coverage**: **100.00%** (234 of 234 functions executed).
- **Clippy**: Clean with 0 warnings on `--all-targets -- -D warnings`.
- **Property-Based Testing**: `proptest` verifying arbitrary RFC 3986 roundtrips and arbitrary struct Serde roundtrips.
