# Std.Async

**Std.Async** provides tasks, structured concurrency helpers, and channels. It builds on the language `async`/`await` model ([Concurrency](../language/12-concurrency.md)).

```ll
import Std.Async
```

## MVP surface (Phase 1.2)

| API | Status |
|-----|--------|
| `await` on `Task` / zero-arg `async` calls / `async let` | Shipped (join / spawn+join) |
| `await` on blocking builtins (`Http.get`, …) | Shipped (runs inline) |
| `async let` + cancel on early exit | MVP (zero-arg async; best-effort cancel) |
| `@run foo()` | Zero-arg free `-> Void` functions |
| `Channel` / `WaitGroup` / `CancellationToken` | Shipped |
| `TaskGroup` | Shipped MVP — see below |
| `MainActor.drain()` | Shipped — see below |
| `withTaskGroup` / valued task-group results | **Later** |
| `withTimeout` / `AsyncSequence` / `runBlocking` | **Later** |

## Task

Joinable handles from `@run` / `async let` / `await` of zero-arg async workers:

```ll
// Fire-and-forget
@run worker()

// Structured: spawn at `async let`, join at `await`
async let work = compute()
await work
```

`Task.cancel()` / `ll_task_cancel` is cooperative — workers must poll a `CancellationToken` or exit.

## CancellationToken

```ll
let token = CancellationToken()           // ctor sugar for `.create()`
token.throwIfCancelled()                  // traps if cancelled
// … work …
token.cancel()
```

Pass the token into cancellable work. There is no `fn.cancel()` on free functions — cancel via the token (or a `Task` handle).

## Channels / WaitGroup

```ll
let ch = Channel.create(4)
ch.send(42)
let v = ch.receive()
ch.close()

let wg = WaitGroup.create()
wg.add(1)
wg.done()
wg.wait()
```

Channel MVP carries `Int` payloads.

## TaskGroup

Structured spawn+join for zero-arg free `-> Void` functions (MVP):

```ll
let g = TaskGroup()       // or TaskGroup.create()
g.add(workerA)            // bare function ref — not workerA()
g.add(workerB)
g.wait()                  // joins all members
```

`TaskGroup.add` spawns each worker into the group; `wait` joins every handle. Capturing closures / returning values / `withTaskGroup` are **Later**.

## MainActor drain vs UI loop

`MainActor.drain()` runs queued main-actor jobs **synchronously until the queue is empty**, then returns. It is a test/CLI helper, not a UI event loop:

- **CLI / agents:** call `MainActor.drain()` after submitting work when you need the main queue flushed before exit.
- **UI apps (Later):** a real run loop would keep draining as events arrive; do not spin `drain()` in a hot loop as a substitute for a platform UI loop.

`$MAIN_ACTOR` is available as a compile-time constant where isolation attributes apply.

## See also

- [Sync](sync.md) — mutexes vs actors
- [Http](http.md) — async client
- [Concurrency](../language/12-concurrency.md)
