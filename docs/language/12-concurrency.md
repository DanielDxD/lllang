# Concurrency

LLLang provides **async/await** with structured concurrency. Tasks are created implicitly at async call sites — function names do not use an `Async` suffix.

**Related:** [Memory model](11-memory-model.md) · [Attributes](14-attributes.md) · [Error handling](10-error-handling.md)

## Async functions

```ll
public async func fetchUser(id: UUID) throws(HttpError) -> User {
    let data = try await Http.get("/users/\(id)")
    try JSON.decode(data)
}
```

Callers must be in an async context or use task builders:

```ll
async func load() {
    let user = try await fetchUser(id)
    display(user)
}
```

## await

`await` must appear in an `async` function. MVP behavior:

| Expression | Effect |
|------------|--------|
| `await task` where `task: Task` | `ll_task_join` |
| `await foo()` zero-arg `async` (void / pointer return) | Spawn worker + join |
| `await foo(args…)` / `await Http.get(…)` | Runs inline (blocking builtins / args MVP) |
| Other values | Error (`LL3001`) — not a silent no-op |

```ll
let a = await computeA()
let b = await computeB()
```

Order is sequential unless combined with `async let` (or Later: task groups).

## Structured concurrency

### async let

```ll
async func loadDashboard() {
    async let stats = fetchStats()   // zero-arg async MVP
    async let feed = fetchFeed()
    let view = Dashboard(await stats, await feed)
    render(view)
}
```

Child tasks are cancelled and joined when the parent returns or the scope ends without awaiting them (best-effort MVP).

### Task groups

`withTaskGroup` / `TaskGroup` are **Later** — not part of the Phase 1.1b surface. Use `async let`, `@run`, `WaitGroup`, and `Channel` instead.

## MainActor and isolation

Custom actors:

```ll
public actor Cache {
    private var store: [String: Data] = [:]
    public func get(_ key: String) -> Data? { store[key] }
    public func put(_ key: String, _ value: Data) { store[key] = value }
}
```

Arguments to actor methods must be `Sendable` (diagnostic `LL2005`).

`MainActor.drain()` flushes the main serial queue **once** (CLI/tests). It is not a UI event loop — see [Std.Async](../stdlib/async.md#mainactor-drain-vs-ui-loop).

## Sendable

Types crossing task boundaries should conform to `Sendable`:

```ll
public struct Config: Sendable {
    public let timeout: Duration
    public let retries: Int
}
```

Classes are `Sendable` only when marked `@unchecked Sendable` with documented internal locking, or when immutable and `final`.

## Cancellation

Tasks inherit cancellation tokens:

```ll
async func poll() async throws {
    while !Task.isCancelled {
        try await tick()
        try await Task.sleep(.milliseconds(100))
    }
}
```

Cooperative cancellation — no forced thread kill.

## Sync vs async overloads

Prefer distinct names only when semantics differ; otherwise overload by async:

```ll
public func load() -> Data { ... }           // sync wrapper may block
public async func load() async throws -> Data { ... }
```

Blocking wrappers should be documented when they hide thread pool usage.

## Mutex and atomics

Low-level primitives live in `Std.Concurrency`:

```ll
let lock = Mutex()
lock.withLock { counter += 1 }
```

Prefer actors over raw locks in application code.

## Thread-local and task-local

`TaskLocal` propagates context (request IDs, tracing):

```ll
TaskLocal.withValue(requestId) {
    await handleRequest()
}
```

## Errors and async

Throwing async functions compose with `try await` and expression `catch`:

```ll
let user = await fetchUser(id) catch (_) => User.guest
```

## Parallelism vs concurrency

CPU-bound parallelism may use worker pools or `parallel for` in std numerics packages. Async is for I/O-bound workloads primarily.

## Deterministic deinit interaction

Objects deallocated on the thread that releases the last reference unless transferred; async callbacks should use `[weak self]` to avoid extending lifetimes unexpectedly.

## Summary

| Topic | LLLang |
|-------|--------|
| Naming | No `Async` suffix |
| Tasks | Implicit at `async` calls |
| UI | `@MainActor` |
| Shared mutable state | `actor` |
| Safety | `Sendable` |

Next: [Pattern matching](13-pattern-matching.md).


## MainActor drain vs UI loop

`$MAIN_ACTOR` / MainActor drain runs queued actor messages on the main executor. There is **no** UI event loop in the toolchain; drain is for CLI/service pumps (`examples/actors-cancel`). GUI frameworks would own their own loop and hop onto MainActor explicitly (Later).
