# Std.Async

**Std.Async** provides tasks, structured concurrency, and channels. It builds on the language `async`/`await` model ([Concurrency](../language/12-concurrency.md)).

```ll
import Std.Async
```

## Task

```ll
public struct Task<Success: Sendable> {
    public static func detached(priority: TaskPriority = .medium, _ operation: @escaping () async throws(E) -> Success) -> Task<Success>
    public static func `do`(priority: TaskPriority = .medium, _ operation: @escaping () async throws(E) -> Success) async throws(E) -> Success
    public func cancel()
    public var isCancelled: Bool { get async }
    public func value() async throws(E) -> Success
}
```

`Task.do` inherits cancellation and task-local context from the caller; `detached` does not.

## Task groups

```ll
public func withTaskGroup<ChildTask: Sendable, GroupResult>(
    returning: GroupResult.Type,
    body: (inout TaskGroup<ChildTask>) async throws(E) -> GroupResult
) async throws(E) -> GroupResult

public struct TaskGroup<Element: Sendable> {
    public mutating func addTask(_ operation: @escaping () async throws(E) -> Element)
    public mutating func next() async throws(E) -> Element?
    public mutating func cancelAll()
}
```

## Channels

```ll
public struct Channel<Element: Sendable> {
    public init(capacity: Int = 0)
    public func send(_ element: Element) async throws(ChannelError)
    public func receive() async throws(ChannelError) -> Element
    public func close()
}
```

Buffered (`capacity > 0`) vs unbuffered (handoff) semantics match async iteration patterns.

## Timeouts and cancellation

```ll
public func withTimeout<T: Sendable>(_ duration: Duration, _ operation: () async throws(E) -> T) async throws(E) -> T

public func withTaskCancellationHandler(
    onCancel: @escaping () -> Void,
    operation: () async throws(E) -> Void
) async throws(E)
```

## Executors

```ll
public enum Executor {
    public static func runOnMain(_ operation: @escaping () async -> Void)
    public static func runBlocking<T>(_ operation: () async throws(E) -> T) throws(E) -> T
}
```

Use `runBlocking` sparingly at FFI or legacy boundaries — prefer native async APIs.

## AsyncSequence

```ll
public protocol AsyncSequence {
    associatedtype Element
    associatedtype AsyncIterator: AsyncIteratorProtocol where AsyncIterator.Element == Element
    func makeAsyncIterator() -> AsyncIterator
}

public func merge<T>(_ sequences: [some AsyncSequence<T>]) -> AsyncStream<T>
```

## See also

- [Sync](sync.md) — mutexes vs actors
- [Http](http.md) — async client
