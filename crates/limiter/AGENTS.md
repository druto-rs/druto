# `limiter` Agent Instructions

## Scope

This crate owns rate limiting, burst throttling, token bucket and sliding window algorithms, storage backends, and
HTTP rate limit headers.

## Design Rules

1. **Precision & Thread Safety**:
    - Rate limit state updates must be atomic and race-condition free under high concurrency.
    - Distributed drivers (e.g. Redis) must execute atomic operations (single roundtrip or atomic Lua scripts).

2. **HTTP Standards Compliance**:
    - Emits accurate `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`, and `Retry-After` headers.
    - Default response status on exhaustion is `429 Too Many Requests`.
