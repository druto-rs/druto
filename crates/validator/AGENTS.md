# `validator` Agent Instructions

## Scope

This crate owns schema validation, input sanitization, built-in validation rules, custom validator registration, and
hierarchical error bag generation.

## Design Rules

1. **Deterministic & Safe Execution**:
    - Validation must fail closed: unvalidated fields must never pass through without explicit schema allowance.
    - Error messages must provide clear, actionable feedback without leaking internal system implementation details.

2. **Performance**:
    - Avoid unnecessary allocations during schema evaluation and string checks.
    - Compile regular expressions once or use pre-compiled regex tables.

