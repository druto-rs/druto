# `events` Crate Context

## Purpose

`events` provides a strongly typed, asynchronous in-process event dispatcher, wildcard pub/sub system, and test double
infrastructure for the Druto framework. It enables decoupled, event-driven communication across framework subsystems
(HTTP lifecycle, authentication, database hooks, mail, and background tasks) and application domain logic without
introducing tight compile-time coupling or side effects during testing.

## Reference Architecture

This crate is the Rust equivalent of:

- **AdonisJS**: `@adonisjs/events` (`Emitter`, `BaseEvent`, `EventsBuffer`, `EmitterLike`)
- **Laravel**: `illuminate/events` (`Dispatcher`, `Event::fake()`, wildcards, `until`)
- **Poppinss**: `@poppinss/hooks` (Lifecycle hook registration and runners)

### Comparative Analysis with Existing Rust Libraries

- **`tokio::sync::broadcast` / `async-broadcast`**:
    - Low-level async channels for fanning out identical cloned messages to active receivers.
    - Lacks a typed handler registry, lifecycle hooks, error accumulation policies, wildcard topic routing, and
      isolated testing fakes with assertion capabilities.
- **`event_emitter_rs`**:
    - String-based event emitter relying on dynamic payloads and reflection-like patterns.
    - Lacks compile-time type safety for event payloads, lacks sequential vs concurrent dispatch controls, lacks
      short-circuiting execution (`until`), and lacks test doubles (`EventFake`).
- **Druto `events`**:
    - Bridges the developer ergonomics of AdonisJS and Laravel with Rust's strict memory safety and concurrency
      guarantees.
    - Combines strongly typed events (`trait Event: Send + Sync + 'static`) with dynamic string-based wildcard pattern
      matching (`"order.*"`).
    - Features dual dispatch modes (deterministic sequential execution and concurrent `join_all` execution).
    - Provides short-circuiting execution (`emit_until`) for intercepting or voting on state transitions.
    - Provides first-class test isolation via `EventFake`, supporting both selective and global event faking with
      ergonomic assertion helpers (`assert_dispatched`, `assert_dispatched_times`, `assert_not_dispatched`,
      `assert_none_dispatched`).
    - Runtime-agnostic: relies purely on `std::future::Future` and standard synchronization primitives (`Arc`,
      `RwLock`).

## Core Responsibilities

1. **Strongly Typed Event Registration & Dispatch**:
    - **Event Contract** (`trait Event: Send + Sync + 'static`): Associates an event type with an immutable identifier,
      topic name via default `fn event_name(&self) -> &'static str { std::any::type_name::<Self>() }`, and
      `fn as_any(&self) -> &dyn Any { self }`.
    - **Zero-Copy Reference Dispatch** (`dispatcher.emit(&event).await` or `dispatcher.emit_serial(&event).await`):
      Dispatches typed events by reference (`&E`) to all listeners, eliminating `Clone` requirements and avoiding heap
      allocations during multi-listener fan-out.
    - **Unified Listener Registration** (`dispatcher.listen::<E, _>(listener) -> ListenerId`):
      Accepts both asynchronous closures (`|event: &E| async move { ... }`) and dedicated structs implementing
      `trait Listener<E: Event>` via a unified `IntoListener<E>` trait.
2. **Deterministic & Concurrency Modes**:
    - **Sequential Dispatch** (`emit` / `emit_serial`): Executes registered listeners in strict registration order.
    - **Concurrent Dispatch** (`emit_concurrent`): Executes all registered listeners concurrently using runtime-agnostic
      future combinators (`join_all`).
3. **Wildcard & String Topic Dispatch**:
    - Supports dynamic string-based event names (e.g. `"order.item.created"`, `"user.registered"`).
    - **Segmented Dot-Delimited Matching**:
        - `*`: Matches exactly one segment (e.g. `"order.*"` matches `"order.created"`, but not `"order.item.created"`).
        - `**`: Matches one or more recursive segments (e.g. `"order.**"` matches `"order.item.created"`).
        - `*` / `**` / `dispatcher.on_any(...)`: Matches all events across the entire application.
    - **Zero-Cost Dynamic Payload**: Wildcard listeners receive `(topic: &str, payload: &dyn Any)`, enabling inspection
      of the topic name or dynamic downcasting (`payload.downcast_ref::<OrderCreated>()`) without serialization.
4. **Short-Circuiting & Halting (`emit_until`)**:
    - `dispatcher.emit_until::<E, B>(&self, &E) -> Result<Option<B>, EventError>`:
        - Uses `std::ops::ControlFlow<B, ()>`: listeners return `Break(val)` to immediately halt execution, skip
          subsequent listeners, and return `Ok(Some(val))`.
        - Normal listeners returning `()` or `Ok(())` automatically convert to `Continue(())`. If no listener breaks,
          returns `Ok(None)`.
5. **Configurable Error Handling & Fault Tolerance**:
    - **Fail-Fast** (`ErrorStrategy::FailFast`, default): Halts execution upon the first listener failure, skipping
      subsequent listeners and returning `Err(EventError::ListenerFailed)`.
    - **Resilient / Collect Errors** (`ErrorStrategy::Collect`): Executes all registered listeners regardless of earlier
      failures, aggregating all encountered errors into `EventError::Aggregate(Vec<EventError>)`.
    - **Observer Hook** (`dispatcher.on_error(handler)`): Centralized callback invoked whenever any listener fails,
      ideal for logging, tracing, or telemetry without changing the error resolution strategy.
6. **Re-entrancy Safety & Snapshot-on-Dispatch**:
    - Takes a snapshot of matching listener handles (`Vec<Arc<dyn ...>>`) under a brief read lock and releases the lock
      **immediately before awaiting any listener futures**.
    - Guarantees zero locks held across `.await` points (`clippy::await_holding_lock` clean).
    - Completely eliminates re-entrant deadlocks when listeners emit nested events or modify registrations.
7. **One-Time Listeners (`once`) & Dynamic Deregistration**:
    - **`once::<E, _>(listener)`**: Listener fires exactly once and is automatically removed for subsequent emissions.
    - **`off(id)` & `clear_listeners::<E>()`**: Explicit deregistration of individual listeners or entire event types.
    - **`listen_if(condition, listener)`**: Conditionally registers a listener based on runtime or configuration flags.
8. **Testing Doubles (`EventFake`)**:
    - **Instance-Scoped Interior Mutability with RAII Restoration**:
      ```rust
      let fake = dispatcher.fake();
      // ... execute domain logic ...
      fake.assert_dispatched::<UserRegistered>();
      fake.assert_dispatched_times::<UserRegistered>(1);
      fake.assert_not_dispatched::<OrderShipped>();
      fake.assert_none_dispatched();
      ```
    - **Selective Faking**: `dispatcher.fake_only::<E>()` fakes only specific event types while letting un-faked events
      dispatch to live listeners normally.
    - Thread-safe isolation across parallel tests (`cargo test`) without process-wide static state.
    - Automatically restores live dispatching when `EventFake` / `FakeGuard` is dropped.

## Boundaries & Non-Responsibilities

- **Not a Distributed Message Queue**: In-process pub/sub only. Distributed message buses, task queues, and persistent
  brokers (Kafka, RabbitMQ, Redis Streams, SQS) belong in `crates/queue`.
- **Not Coupled to Framework Subsystems**: Independent of HTTP requests, SQL queries, or mailing contracts.
- **Zero Runtime Dependencies**: Zero hard dependencies on concrete async runtimes (Tokio, async-std) in
  `[dependencies]`. Operates purely on `std::future::Future`, `std::pin::Pin`, and standard synchronization primitives.

## Implemented Public Surface

### Core Traits & Structures

- `trait Event: Send + Sync + 'static`:
    - `fn event_name(&self) -> &'static str` (default: `std::any::type_name::<Self>()`)
    - `fn as_any(&self) -> &dyn Any where Self: Sized` (default: `self`)
- `type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>`: Zero-dependency heap-pinned future type alias.
- `trait Listener<E: Event>: Send + Sync + 'static`:
    - `fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<(), EventError>>`
- `trait TopicListener: Send + Sync + 'static`:
    - `fn handle<'a>(&'a self, topic: &'a str, payload: &'a dyn Any) -> BoxFuture<'a, Result<(), EventError>>`
- `trait UntilListener<E: Event, B>: Send + Sync + 'static`:
    - `fn handle<'a>(&'a self, event: &'a E) -> BoxFuture<'a, Result<ControlFlow<B, ()>, EventError>>`
- `trait IntoListener<E: Event>`:
    - Blanket implementations for async closures (`Fn(&E) -> BoxFuture<Result<(), EventError>>`), infallible async
      closures (`Fn(&E) -> BoxFuture<()>`), and `Listener<E>` structs.
- Functional Listener Adapters:
    - `from_fn(f)`: Wraps fallible async closure into a typed listener.
    - `from_infallible_fn(f)`: Wraps infallible async closure into a typed listener.
    - `from_topic_fn(f)`: Wraps topic closure `(topic, payload)` into a `TopicListener`.
    - `from_until_fn(f)`: Wraps short-circuiting closure into an `UntilListener`.
- `struct EventDispatcher`: Thread-safe, cloneable dispatcher wrapping internal state in `Arc`.
- `struct ListenerId(u64)`: Unique handle for registered listeners.
- `struct EventFake`: Test double tracking dispatched events and providing rich assertions.
- `enum ErrorStrategy`: `FailFast` (default) vs `Collect`.
- `enum EventError`: Strongly typed errors covering `ListenerFailed`, `Aggregate`, `Custom`, and downcast mismatches.
- `fn matches_topic(pattern: &str, topic: &str) -> bool`: Segmented wildcard topic matcher (`*` single segment, `**`
  recursive multi-segment).
- `struct ConcurrentFutures<F>`: Zero-dependency concurrent future runner driving polling without external async
  runtimes.

### Dispatcher API

- `dispatcher.listen::<E, L>(&self, listener: L) -> ListenerId`: Register typed listener (`IntoListener<E>` adapter or
  struct).
- `dispatcher.listen_fn::<E, F>(&self, handler: F) -> ListenerId`: Register typed fallible or infallible async closure
  directly.
- `dispatcher.listen_topic<L>(&self, pattern: &str, listener: L) -> ListenerId`: Register wildcard/topic listener via
  struct or adapter.
- `dispatcher.listen_topic_fn<F>(&self, pattern: &str, handler: F) -> ListenerId`: Register wildcard/topic async closure
  directly.
- `dispatcher.on_any<L>(&self, listener: L) -> ListenerId`: Register catch-all wildcard (`**`) listener.
- `dispatcher.on_any_fn<F>(&self, handler: F) -> ListenerId`: Register catch-all wildcard (`**`) async closure.
- `dispatcher.once::<E, L>(&self, listener: L) -> ListenerId`: Register one-time listener.
- `dispatcher.once_fn::<E, F>(&self, handler: F) -> ListenerId`: Register one-time async closure.
- `dispatcher.listen_if::<E, L>(&self, condition: bool, listener: L) -> Option<ListenerId>`: Conditionally register
  listener.
- `dispatcher.listen_if_fn::<E, F>(&self, condition: bool, handler: F) -> Option<ListenerId>`: Conditionally register
  async closure.
- `dispatcher.listen_until::<E, B, L>(&self, listener: L) -> ListenerId`: Register short-circuiting listener.
- `dispatcher.listen_until_fn::<E, B, F>(&self, handler: F) -> ListenerId`: Register short-circuiting async closure.
- `dispatcher.off(&self, id: ListenerId) -> bool`: Unregister a listener by ID.
- `dispatcher.clear_listeners::<E>(&self)`: Clear all typed listeners for an event type.
- `dispatcher.clear_all_listeners(&self)`: Clear all listeners across all types and topics.
- `dispatcher.has_listeners::<E>(&self) -> bool`: Check if listeners exist for an event.
- `dispatcher.listener_count::<E>(&self) -> usize`: Return count of registered listeners for an event.
- `dispatcher.emit::<E>(&self, event: &E) -> Result<(), EventError>`: Sequential dispatch.
- `dispatcher.emit_serial::<E>(&self, event: &E) -> Result<(), EventError>`: Explicit sequential dispatch.
- `dispatcher.emit_concurrent::<E>(&self, event: &E) -> Result<(), EventError>`: Concurrent dispatch driving futures
  cooperatively.
- `dispatcher.emit_until::<E, B>(&self, event: &E) -> Result<Option<B>, EventError>`: Short-circuiting dispatch
  returning first `Break(val)`.
- `dispatcher.emit_topic(&self, topic: &str, payload: &dyn Any) -> Result<(), EventError>`: Dynamic topic dispatch.
- `dispatcher.on_error<F>(&self, handler: F)`: Register global listener failure observer hook.
- `dispatcher.fake(&self) -> EventFake`: Activate full test double intercepting all events.
- `dispatcher.fake_only::<E>(&self) -> EventFake`: Activate selective test double for specific event types.
- `dispatcher.fake_topics(&self, patterns: &[&str]) -> EventFake`: Activate selective test double for specific topic
  patterns.
- `dispatcher.restore(&self)`: Restore live dispatching explicitly (also done automatically on `EventFake` drop).
- `dispatcher.is_faking(&self) -> bool`: Check if dispatcher is currently faking events.

### Testing Double API (`EventFake`)

- `fake.assert_dispatched::<E>()`: Assert event type was dispatched at least once.
- `fake.assert_dispatched_times::<E>(count)`: Assert event type was dispatched exact number of times.
- `fake.assert_not_dispatched::<E>()`: Assert event type was not dispatched.
- `fake.assert_none_dispatched()`: Assert no events of any kind were dispatched.
- `fake.assert_dispatched_topic(topic)`: Assert specific topic string was emitted.
- `fake.assert_not_dispatched_topic(topic)`: Assert specific topic string was not emitted.
- `fake.dispatched_count::<E>() -> usize`: Get count of dispatched events for type.
- `fake.total_dispatched() -> usize`: Get total count of dispatched events.
- `fake.dispatched_topics() -> Vec<String>`: List all emitted topic names.
- `fake.on_dispatched::<E, F>(&self, inspector: F)`: Register inspection callback for dispatched instances.
- `fake.flush()`: Clear all recorded events.
- `fake.restore()`: Explicitly end faking (also invoked on `Drop`).
