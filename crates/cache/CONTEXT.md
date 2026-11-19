# `cache` Crate Context & Architecture

## Vision & Scope

The `cache` crate provides a unified, high-performance, multi-driver caching engine for the Druto web framework.
It combines sub-microsecond in-memory performance with horizontally scalable distributed caching, resilient
stampede defense, stale-while-revalidate (SWR) grace periods, tag-based invalidation, and cross-node cache
synchronization. It operates both as an independent caching substrate across background workers and services, and as
an integrated layer within HTTP handlers, middlewares, and repositories.

## Reference Blueprint & Specification Mapping

The architecture synthesizes high-concurrency caching patterns, distributed systems principles, and official standards:

- **RFC Specifications**:
    - **RFC 5861**: HTTP Cache-Control Extensions for Stale Content (`stale-while-revalidate` and `stale-if-error`).
      Guides logical versus physical entry expiration, background revalidation, and graceful degradation during
      upstream failures.
    - **RFC 7234 / RFC 9111**: HTTP Caching freshness models, delta-seconds calculation, and explicit invalidation
      semantics.
- **Multi-Tier Hybrid Storage**:
    - Two-tier storage composition: L1 local in-memory fast tier backed by an L2 distributed remote tier (e.g. Redis).
    - Asynchronous synchronization bus: Cluster-wide L1 cache eviction over distributed Pub/Sub channels on key
      mutation.
- **Cache Stampede Defense (Single-Flight)**:
    - In-flight request coalescing on cache misses: when multiple concurrent threads or tasks request a missing or
      expired key simultaneously, exactly one factory execution is scheduled while other callers await the shared
      result.
- **Lightweight Timestamp-Based Tag Invalidation**:
    - $O(1)$ tag invalidation vector algorithm: cache entries record metadata (`created_at` and associated tags). Tag
      invalidation stamps a monotonically increasing timestamp onto the tag key, eliminating volatile inverted index
      sets
      and complex race conditions.

## Key Capabilities & Core Responsibilities

1. **Storage Backend Abstraction (`CacheStore`)**:
    - Runtime-agnostic, object-safe, thread-safe asynchronous contract: `trait CacheStore: Send + Sync + 'static`.
    - Core driver operations:
        - `get(&self, key: &str) -> BoxFuture<'_, Result<Option<CacheEntry>, CacheError>>`
        - `put(&self, key: &str, entry: &CacheEntry, ttl: Duration) -> BoxFuture<'_, Result<(), CacheError>>`
        - `put_many(&self, entries: &[(&str, &CacheEntry, Duration)]) -> BoxFuture<'_, Result<(), CacheError>>`
        - `add(&self, key: &str, entry: &CacheEntry, ttl: Duration) -> BoxFuture<'_, Result<bool, CacheError>>`
        - `pull(&self, key: &str) -> BoxFuture<'_, Result<Option<CacheEntry>, CacheError>>`
        - `forget(&self, key: &str) -> BoxFuture<'_, Result<bool, CacheError>>`
        - `forget_many(&self, keys: &[&str]) -> BoxFuture<'_, Result<usize, CacheError>>`
        - `increment(&self, key: &str, amount: i64) -> BoxFuture<'_, Result<i64, CacheError>>`
        - `decrement(&self, key: &str, amount: i64) -> BoxFuture<'_, Result<i64, CacheError>>`
        - `flush(&self) -> BoxFuture<'_, Result<(), CacheError>>`

2. **Concrete Storage Drivers**:
    - **`MemoryCacheStore`**: In-process concurrent memory cache bounded by entry count or weighted memory size,
      supporting TinyLFU/LRU admission-eviction algorithms, per-entry expiration, and sub-microsecond access latency.
    - **`RedisCacheStore`**: High-performance distributed cache backed by `crates/redis` (`RedisCommands`). Employs
      atomic Lua scripts for compound checks (`add`, `pull`, tag verification) and relies on native Redis TTL eviction
      (`SETEX`, `EXPIRE`).
    - **`TieredCacheStore`**: Hierarchical L1/L2 multi-tier cache. Reads hit L1 first; on L1 miss, reads fetch from L2
      and backfill L1. Writes propagate to both L2 and L1.
    - **`FileCacheStore`**: Filesystem-backed persistent storage utilizing atomic temporary file creation and atomic
      renames (`std::fs::rename`) to guarantee crash resilience.
    - **`NullCacheStore`**: Zero-allocation no-op implementation for test environments or selectively disabling caching.

3. **High-Level `Cache` Facade & Developer Ergonomics**:
    - Strongly typed serialization and deserialization via `serde` (`Serialize`, `DeserializeOwned`).
    - Methods:
        - `get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError>`
        - `put<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<(), CacheError>`
        - `remember<T, F, Fut>(&self, key: &str, ttl: Duration, factory: F) -> Result<T, CacheError>`
        - `remember_forever<T, F, Fut>(&self, key: &str, factory: F) -> Result<T, CacheError>`
        - `pull<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError>`
        - `add<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<bool, CacheError>`
        - `has(&self, key: &str) -> Result<bool, CacheError>`
        - `forget(&self, key: &str) -> Result<bool, CacheError>`
        - `tags(&self, tags: &[&str]) -> TaggedCache`
        - `namespace(&self, prefix: &str) -> NamespacedCache`

4. **Cache Stampede Defense (Single-Flight)**:
    - Prevents database or upstream service overload during cold starts or key expirations.
    - An in-flight task coordinator monitors pending keys. Multiple concurrent callers awaiting the same key join the
      single active future, receiving clones of the resolved value without duplicate upstream invocations.

5. **Soft Expiration, Grace Periods & Stale-While-Revalidate (SWR)**:
    - Decouples **logical expiration** (when an entry is considered stale) from **physical storage expiration**
      (`logical_ttl + grace_duration`).
    - If a request encounters a logically expired entry within its grace window:
        - **Stale-While-Revalidate**: Immediately serves the stale cached value while scheduling an asynchronous
          background revalidation task.
        - **Stale-If-Error**: If upstream factory execution fails or times out, safely falls back to the graced value
          instead of bubbling errors to users.

6. **Timestamp-Based Tag Invalidation**:
    - Supports tagging related entries (e.g., `tags(["users", "teams"])`).
    - Avoids maintaining inverted index sets (which cause lock contention, memory leaks, and distributed race
      conditions).
    - Invalidation writes a timestamp key `__tag:<tag_name>` with the current Unix millisecond timestamp.
    - Entry evaluation checks whether `entry.created_at <= tag_invalidated_at`. If superseded, the entry is treated
      as expired on read and pruned.

7. **Cross-Node Synchronization Bus**:
    - Optional distributed invalidation bus for multi-node deployments backed by `crates/redis` pub/sub.
    - When an L2 write or delete occurs on Node A, a pub/sub eviction message broadcasts across cluster nodes,
      evicting the matching key from Node B's L1 memory cache.

8. **Telemetry & Event Dispatching**:
    - Emits strongly typed events via `crates/events`:
        - `CacheHit`: Emitted on successful retrieval (annotated with tier: L1, L2, or Graced).
        - `CacheMiss`: Emitted when key is absent or expired.
        - `KeyWritten`: Emitted on key write/update.
        - `KeyForgotten`: Emitted on key deletion or invalidation.
