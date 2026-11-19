# `limiter` Crate Context & Architecture

## Vision & Scope

The `limiter` crate provides flexible rate limiting, traffic throttling, burst control, and standardized HTTP rate limit
response headers for the Druto web framework. It protects endpoints from brute force attacks, scraping, and excessive
load.

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/limiter` (multi-store rate limiting, sliding window, memory/redis/database stores)
- **Laravel**: `Illuminate\Cache\RateLimiter` + `Illuminate\Routing\Middleware\ThrottleRequests`
- **IETF RFC**: RFC 6585 (429 Too Many Requests) and IETF RateLimit Header Field drafts

## Key Capabilities & Responsibilities

1. **Algorithms**:
    - **Token Bucket**: Allows bursts up to a capacity, refilling tokens continuously at a steady rate.
    - **Fixed Window**: Tracks request counts within discrete time intervals (e.g. 100 requests per minute).
    - **Sliding Window**: Smooths out boundary traffic spikes across rolling time frames.

2. **Storage Drivers**:
    - **In-Memory Store**: Thread-safe, atomic, zero-allocation lock-free or dashmap-backed store for standalone nodes.
    - **Redis Store**: Distributed atomic rate limiting across horizontally scaled instances using Lua scripts.

3. **Key Resolution & Derivation**:
    - Flexible key generators: by client IP address, authenticated user ID, route pattern, API key, or custom closure.
    - Granular throttling tiers (e.g. VIP accounts vs anonymous visitors).

4. **HTTP Header Compliance**:
    - Standard RFC / IETF headers: `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`.
    - Legacy headers: `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`, `Retry-After`.

5. **Middleware Integration**:
    - Implements `pipeline::Middleware<HttpContext, Response, RouterError>`.
    - Immediate `429 Too Many Requests` responses or configurable custom rejection handlers.
