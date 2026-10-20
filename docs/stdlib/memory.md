# Std.Memory

**Std.Memory** provides explicit allocation strategies — **arenas** and **pools** — for hot paths where ARC and general-purpose heap allocation are too costly.

```ll
import Std.Memory
```

Language baseline: [Memory model](../language/11-memory-model.md).

## Arena

Bump-pointer allocator; reset or drop the whole arena to reclaim memory at once.

```ll
public struct Arena {
    public init(initialBytes: Int = 65536)
    public mutating func allocate(byteCount: Int, alignment: Int = 8) -> Pointer<UInt8>
    public mutating func allocate<T>(_ type: T.Type, count: Int = 1) -> Pointer<T>
    public mutating func reset()
    public mutating func freeze() -> FrozenArena
}
```

```ll
public struct FrozenArena: Sendable {
    public func withUnsafeBytes<R>(_ body: (Span<UInt8>) throws(E) -> R) throws(E) -> R
}
```

Typical pattern: parse or serialize inside `withArena { arena in ... }`, return only copied `Data`/`String` to safe code.

## Pool

Fixed-size object pool for structs of uniform size.

```ll
public struct Pool<T> {
    public init(capacity: Int, factory: () -> T)
    public mutating func acquire() -> Pooled<T>
}

public struct Pooled<T> {
    public var value: T { get mutating set }
    public mutating func release(to pool: inout Pool<T>)
}
```

## Scratch buffer

Thread-local reusable buffer for formatting and IO:

```ll
public enum Scratch {
    public static func withBuffer<R>(capacity: Int = 4096, _ body: (inout [UInt8]) throws(E) -> R) throws(E) -> R
}
```

## Scope helpers

```ll
public func withArena<R>(_ body: (inout Arena) throws(E) -> R) throws(E) -> R
```

## Safety

Pointers from arenas must not escape the arena lifetime unless copied into `Data` or other owned types. The compiler aids diagnostics when arena pointers are stored in long-lived structs without `unsafe` annotation.

## See also

- [Json](json.md) — parse into arena-backed docs
- [FFI](ffi.md) — pass arena slices to C with documented lifetimes
