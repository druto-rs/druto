# `matcher` Crate Context

## Purpose

`matcher` is a high-performance URL pattern matching, parameter extraction, and HTTP method routing engine for the Druto
web framework. It provides the algorithmic core for compiling route patterns, indexing them in a deterministic radix
trie, and matching incoming URIs with zero fast-path heap allocations.

## Core Responsibilities

1. **Pattern Compilation & Grammar**:
    - Dual parameter syntax: colon style (`:id`, `:id?`, `*wildcard`) and brace style (`{id}`, `{id?}`, `{wildcard*}`).
    - Inline constraint shortcuts: `:id(num)`, `:id(uuid)`, `:id(slug)`, `:id(alpha)`, `:id(alphanumeric)`,
      `:id(/regex/)`, and `{id:num}`, `{id:[0-9]+}`.
    - Programmatic constraint assignment via fluent `RouteBuilder::where(...)`.
    - Strict pattern validation: catches trailing segments after wildcards, duplicate parameter names within a pattern,
      and required parameters following optional parameters.

2. **Deterministic Route Trie Indexing**:
    - Segment-based radix trie (`RouteNode<T>`) storing arbitrary generic payload `T`.
    - Deterministic precedence hierarchy (independent of route registration order):
        1. Exact static literal segments (e.g., `/users/profile`).
        2. Regular expression constrained parameters (highest specific priority).
        3. Built-in type constrained parameters (ordered: `Uuid` > `Num` > `Slug` > `Alpha` / `Alphanumeric`).
        4. Unconstrained dynamic parameters (e.g., `:id`).
        5. Wildcard catch-all segments (e.g., `*rest`, `**`).
    - Registration-time collision detection: ambiguous overlapping routes with identical specificity fail immediately
      with `MatcherError::ConflictingRoute`.

3. **Zero-Allocation Fast-Path Matching**:
    - Up to 16 path parameters are stored in an inline stack array `[(&'a str, &'a str); 16]` with heap overflow
      fallback.
    - Parameter values borrow directly as `&'a str` slices from the input URI path.
    - On-demand zero-allocation percent decoding (`params.get_decoded("key")` returns `Cow<'a, str>`, allocating only if
      percent characters require transformation).
    - Typed conversion helper: `params.parse::<U>("key")` using standard `FromStr`.
    - Query string extraction: `QueryParams<'a>` borrows keys and values from the URI query section with on-demand
      decoding.

4. **HTTP Method Routing & RFC Compliance**:
    - `MethodMatcher<T>` indexes routes by HTTP verb (`Method`) alongside an `ANY` wildcard table.
    - Full verb support: RFC 9110 standard methods (`GET`, `POST`, `PUT`, `DELETE`, `PATCH`, `HEAD`, `OPTIONS`,
      `CONNECT`, `TRACE`), RFC 10008 `QUERY`, WebDAV (`PROPFIND`, `PROPPATCH`, `MKCOL`, `COPY`, `MOVE`, `LOCK`,
      `UNLOCK`), CalDAV/CardDAV/Search (`REPORT`, `MKCALENDAR`, `SEARCH`, `ACL`), CDN/caching (`PURGE`, `BAN`), and
      custom extension verbs.
    - Built-in RFC 9110 semantic predicates: `method.is_safe()` and `method.is_idempotent()`.
    - Automatic 405 Method Not Allowed calculation: when a URI path exists for other methods, returns all allowed verbs.
    - Automatic RFC 9110 `HEAD` fallback to `GET` handler when explicit `HEAD` route is omitted.

5. **Reverse URL Generation**:
    - Named route registration (`Matcher::insert_named`, `MethodMatcher::insert_named`).
    - Reverse path generation (`build_path`) validating required parameters, omitting optional parameters if unset, and
      checking defined constraints.

## Boundaries & Non-Responsibilities

- **Pure In-Memory Logic**: Completely synchronous, zero network I/O, no async runtime dependencies.
- **Handler Agnostic**: Does not know about HTTP bodies, headers, status code serialization, or middleware chains;
  routes store generic payload `T`.
- **Framework Plumbing**: Higher-level request/response handling and middleware pipelines belong to subsequent framework
  layers.

## Public Surface

- `Matcher<T>`: Pure path radix trie routing container.
- `Match<'a, T>`: Match result providing `&'a T` payload and borrowed `Params<'a>`.
- `RouteBuilder<'m, T>`: Fluent registration builder for parameter constraints and naming.
- `MatcherConfig`: Configuration options for flexible trailing slashes and case sensitivity.
- `MethodMatcher<T>`: HTTP method-aware routing container with 405 tracking and HEAD fallback.
- `MethodMatch<'a, T>`: Method match result containing payload, `Params<'a>`, and matched `Method`.
- `Method`: Comprehensive HTTP method enum with standard, extended, WebDAV, and custom variants.
- `Params<'a>`: Stack-allocated path parameter map with zero-alloc slicing and decoded views.
- `QueryParams<'a>`: Borrowed query string parameter iterator and lookup map.
- `RouteParser`: Pattern parser emitting compiled `Vec<Segment>`.
- `Segment`: Structural AST segment variants (`Static`, `Param`, `Wildcard`).
- `Constraint` & `BuiltinConstraint`: Validation constraints for dynamic path segments.
- `MatcherError`: Domain errors for parsing, route collisions, constraint failures, and missing parameters.
