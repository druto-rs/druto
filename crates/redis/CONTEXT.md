# `redis` Crate Context & Architecture

## Vision & Scope

The `redis` crate provides asynchronous connection management, multiplexed connection pooling, pub/sub messaging
channels,
cluster/sentinel awareness, and Redis command pipeline abstractions for the Druto web framework. It acts as the
high-throughput
distributed infrastructure backend for sessions, rate limiting, caching, and background queues.

## Reference Blueprint & Equivalent Mapping

- **AdonisJS**: `@adonisjs/redis` (connection manager, pub/sub, sentinel, clustering)
- **Laravel**: `Illuminate\Redis` (`RedisManager`, connection pooling, transactions, pub/sub)
- **Rust Ecosystem**: `fred` / `redis-rs` connection architectures

## Key Capabilities & Responsibilities

1. **Connection Management & Pooling**:
    - Multi-connection configuration (named connections e.g. `default`, `cache`, `subscriber`).
    - Multiplexed asynchronous connection pool with automatic reconnection and exponential backoff.
    - Standalone, Redis Sentinel, and Redis Cluster topology support.

2. **Command Pipeline & Transactions**:
    - Idiomatic async API for standard key-value, hash, set, sorted set, list, and stream commands.
    - Atomic MULTI/EXEC transaction blocks and pipelined batch execution.
    - Lua script evaluation with automatic SHA1 caching (`EVALSHA`).

3. **Pub/Sub Messaging**:
    - Dedicated asynchronous subscriber channels with pattern matching (`PSUBSCRIBE`).
    - Clean integration with Druto's `events` system.

4. **Framework Driver Provider**:
    - Provides underlying atomic storage primitives for `crates/session` (Redis session store), `crates/limiter`
      (sliding window rate limiter), and future cache drivers.
