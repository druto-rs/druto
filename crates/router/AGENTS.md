# `router` Agent Instructions

## Scope

This crate owns the HTTP routing engine, request/response models, middleware onion runner, and execution context
(`HttpContext`).
It bridges `matcher`, `pipeline`, `cookie`, `qs`, `crypto`, and `session` without coupling directly to concrete
transport socket listeners.

## Design Rules

1. **Transport & Runtime Agnostic**:
    - The router must not depend directly on concrete transport socket listeners or be tightly coupled to an async
      runtime.
    - It processes abstract HTTP requests and produces abstract HTTP responses.
2. **Deterministic Precedence**:
    - Route compilation must rely on `matcher::MethodMatcher` to guarantee deterministic specificity hierarchy
      regardless of registration order.
3. **Immutability After Compilation**:
    - Route registration happens in a mutable builder phase. Dispatching requests must operate on an immutable,
      thread-safe (`Send + Sync + 'static`) compiled router.
4. **Middleware Onion Discipline**:
    - Middleware execution must integrate with `pipeline::Pipeline`, allowing global, group, and route-specific
      middleware to intercept requests and wrap responses.
5. **No Clippy Suppressions**:
    - Do not suppress Clippy warnings inline (`#[allow(...)]` or `#[expect(...)]`). Resolve the warning or restructure
      the code instead.
6. **No Leading Underscores**:
    - Do not use leading underscores on identifiers, variables, parameters, or struct fields.
7. **Comments**:
    - Comment only the "why" when code alone cannot state the external constraint or intentional design decision.
      Document public surfaces with rustdoc (`///`, `//!`).
8. **Reverse URL Resolution**:
    - Reverse URL generation must strictly validate required parameters and URL-encode dynamic values.
