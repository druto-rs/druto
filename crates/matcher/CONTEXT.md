# `matcher` Crate Context

## Purpose

`matcher` is a high-performance URL pattern matching and parameter extraction engine for the Druto web framework. It provides the algorithmic core for routing HTTP requests to handlers based on path patterns.

## Reference Architecture

This crate is the Rust equivalent of:
- **AdonisJS**: `@poppinss/matchit` and `@adonisjs/http-server/src/router`
- **Laravel**: `illuminate/routing` route compiler and matching algorithms
- **Prior Rust art**: `matchit`

Unlike generic regex-based routers, `matcher` is designed around a deterministic segment tree (radix/prefix tree) tailored specifically for URI paths.

## Core Responsibilities

1. **Pattern Compilation**:
   - Parse path pattern strings into discrete segment descriptors.
   - Support static segments: `/users`, `/api/v1/posts`.
   - Support named parameter segments: `:id`, `{id}`.
   - Support wildcard / catch-all segments: `*path`, `**`.
   - Support optional segments: `:id?`.
   - Support segment validation constraints (e.g., regex or type validators like `:id(num)`).

2. **Route Tree Construction**:
   - Store registered routes associated with generic payload types `T` (e.g., route IDs, handler pointers, or metadata).
   - Maintain strict, predictable precedence rules:
     1. Exact static matches (e.g., `/users/me`) take priority over parameterized matches.
     2. Constrained parameters take priority over unconstrained parameters.
     3. Parameterized matches (e.g., `/users/:id`) take priority over wildcards.
     4. Catch-all / wildcards (e.g., `/users/*rest`) are matched last.

3. **Path Matching & Extraction**:
   - Match an incoming path string against the tree.
   - Extract parameter key-value pairs with minimal allocations (preferring string slices `&str` referencing the input path).
   - Return route payload `&T` along with extracted parameters.

## Boundaries & Non-Responsibilities

- **HTTP Agnostic**: Does not know about HTTP methods (`GET`, `POST`), request headers, query strings, or body payloads.
- **Handler Agnostic**: Does not know what a handler or middleware is; it stores generic payload `T`.
- **Zero Network I/O**: Completely synchronous, pure memory data structure.

## Target Public Surface

- `Matcher<T>`: The route tree container.
- `Match<'a, T>`: Result of a successful match containing `&'a T` and `Params<'a>`.
- `Params<'a>`: Key-value parameter collection with lookup by name (`params.get("id")`) and iteration.
- `Segment`: Internal or public AST representation of route segments.
- `MatcherError`: Parsing and registration error types (e.g., conflicting routes, invalid syntax).
