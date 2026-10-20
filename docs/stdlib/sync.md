# Std.Sync

**Std.Sync** offers locks and semaphores for low-level threading when actors are too heavy. Prefer `actor` types and `@MainActor` when possible ([Concurrency](../language/12-concurrency.md)).

```ll
import Std.Sync
```

## Mutex

```ll
public final class Mutex: Sendable {
    public init()
    public func withLock<R>(_ body: () throws(E) -> R) throws(E) -> R
}
```

## Read-write lock

```ll
public final class RWLock: Sendable {
    public init()
    public func withReadLock<R>(_ body: () throws(E) -> R) throws(E) -> R
    public func withWriteLock<R>(_ body: () throws(E) -> R) throws(E) -> R
}
```

## Semaphore and condition

```ll
public final class Semaphore: Sendable {
    public init(value: Int)
    public func wait() throws(SyncError)
    public func signal()
}

public final class Condition: Sendable {
    public init()
    public func wait(mutex: Mutex) throws(SyncError)
    public func notifyOne()
    public func notifyAll()
}
```

## One-shot and barriers

```ll
public final class Once: Sendable {
    public init()
    public func run(_ body: () -> Void)
}
```

## Atomics (selected)

```ll
public struct AtomicInt {
    public init(_ value: Int = 0)
    public func load(order: MemoryOrder = .sequentiallyConsistent) -> Int
    public func store(_ value: Int, order: MemoryOrder = .sequentiallyConsistent)
    public func compareExchange(expected: Int, desired: Int) -> (exchanged: Bool, current: Int)
}
```

## See also

- [Async](async.md) — task cancellation vs locks
- [Memory](memory.md) — pools with lock striping
