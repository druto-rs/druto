# `redis` Agent Instructions

## Scope

This crate owns Redis client connection pooling, command execution pipelines, pub/sub channels, transactions, and
distributed driver integrations.

## Design Rules

1. **Multiplexing & Connection Safety**:
    - Manage multiplexed async connections with graceful reconnection handling.
    - Pub/Sub connections must be isolated from standard command/query connection pools.

2. **Atomic Operations**:
    - Atomic mutations across multiple keys must utilize atomic pipelines, transactions, or Lua scripts.
