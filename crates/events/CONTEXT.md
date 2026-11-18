# `events` Crate Context

## Purpose

`events` provides a strongly typed, asynchronous event dispatcher and pub/sub system for the Druto framework.
It enables decoupled communication across framework subsystems (HTTP lifecycle, database hooks, authentication,
background
jobs) and includes built-in test doubles (`EventFake`) for verifying event dispatch without side effects.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/events` (`Emitter`)
- **Laravel**: `illuminate/events` (`Dispatcher`, `Event::fake()`)
- **Poppinss**: `@poppinss/hooks`

## Core Responsibilities

1. **Strongly Typed Event Registration & Dispatch**:
    - Generic event contract (`trait Event: Send + Sync + 'static`) associating events with an immutable name or topic.
    - Type-safe dispatch (`dispatcher.emit(event).await`) and typed listener registration
      (`dispatcher.listen::<MyEvent, _>(handler)`).
2. **Asynchronous Execution & Concurrency**:
    - Listeners execute asynchronously across multi-threaded runtimes without blocking worker threads.
    - Sequential listener execution order per event, allowing controlled state progression.
3. **Wildcard & String Topic Dispatch**:
    - Supports subscribing to string-based event patterns with wildcard segments (`"order.*"`, `"user.registered"`).
4. **Error Handling & Fault Tolerance**:
    - Configurable error policies: fail-fast (bubbling up listener errors) or resilient (logging and continuing
      remaining listeners).
5. **Testing Doubles (`EventFake`)**:
    - Allows unit and integration tests to swap the dispatcher with an in-memory recorder:
      ```rust
      let fake = dispatcher.fake();
      // ... execute business logic ...
      fake.assert_dispatched::<UserRegistered>();
      fake.assert_not_dispatched::<OrderShipped>();
      ```
    - Prevents real side effects (e.g., sending emails, external webhooks) during test runs while guaranteeing event
      assertion capabilities.

## Boundaries & Non-Responsibilities

- **Not a Distributed Message Broker**: In-process pub/sub only. Distributed queues (Kafka, RabbitMQ, SQS) belong in
  `crates/queue`.
- **Decoupled from Framework Subsystems**: Does not know about HTTP requests, SQL queries, or mailing contracts.

## Proposed Public Surface

- `trait Event`: Core event definition trait with static or runtime name.
- `struct EventDispatcher`: Thread-safe, async event dispatcher.
- `struct EventFake`: Test double with assertion helpers (`assert_dispatched`, `assert_not_dispatched`,
  `dispatched_count`).
- `trait Listener<E: Event>`: Asynchronous event listener contract.
- `enum EventError`: Strongly typed errors for listener execution failures.
