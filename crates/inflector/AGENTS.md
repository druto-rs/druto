# `inflector` Agent Instructions

## Scope

This crate owns string transformations, grammatical pluralization/singularization, and casing conventions for Druto. It
is pure computation with zero network, filesystem, or async dependencies.

## Design Rules

1. **Deterministic & Idiomatic**:
    - Transformations must be deterministic and handle unicode boundary slicing safely without panicking.
2. **Extensive Grammar Rule Tables**:
    - Maintain comprehensive regular and irregular rule tables for English inflection.
    - Support case preservation (e.g. `User` pluralizes to `Users`, `user` to `users`, `USER` to `USERS`).
3. **Allocation Awareness**:
    - Provide zero-allocation checks (e.g., `is_singular`, `is_plural`, casing predicates) where possible.
    - For casing conversions where input already conforms to the target casing, return `Cow::Borrowed` to eliminate
      allocations.
4. **Shared Internal Helpers**:
    - Consolidate common string processing, delimited ASCII scanning, title iteration, and padding buffer allocations
      into internal helpers (`case::mod` and `utils::pad`) rather than duplicating loops or capacity logic across
      modules.

## Testing Requirements

- **Pluralization / Singularization Round-Tripping**:
    - Test extensive lists of regular and irregular nouns (`person` $\to$ `people` $\to$ `person`, `cactus` $\to$
      `cacti` $\to$ `cactus`).
    - Test uncountable words (`money`, `rice`, `information`, `equipment`).
- **Casing Edge Cases**:
    - Strings with consecutive uppercase letters (acronyms like `HTTPResponse` $\to$ `http_response` vs `httpResponse`).
    - Leading, trailing, and duplicate separators (`__hello__world__` $\to$ `hello_world`).
    - Unicode characters and accents in slugification.
