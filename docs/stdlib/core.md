# Std.Core

**Std.Core** is implicit: every compilation unit sees its types and protocols without an import. It defines language-adjacent primitives, universal protocols, and small utilities used across the stdlib.

## Optionals and results

```ll
public enum Optional<T> {
    case none
    case some(T)
}

public enum Result<T, E: Error> {
    case ok(T)
    case err(E)
}
```

```ll
public protocol Error {
    var message: String { get }
}

public enum Never {}
```

## Comparison and hashing

```ll
public protocol Equatable {
    func ==(lhs: Self, rhs: Self) -> Bool
}

public protocol Hashable: Equatable {
    func hash(into hasher: inout Hasher)
}

public struct Hasher {
    public mutating func combine(_ value: some Hashable)
    public func finalize() -> UInt64
}
```

## Ordering

```ll
public protocol Comparable: Equatable {
    func <(lhs: Self, rhs: Self) -> Bool
}
```

## Identifiable and CustomStringConvertible

```ll
public protocol Identifiable {
    associatedtype ID: Hashable
    var id: ID { get }
}

public protocol CustomStringConvertible {
    var description: String { get }
}
```

## Sendable and concurrency markers

```ll
public protocol Sendable {}

@MainActor
public protocol MainActorIsolated {}
```

See [Concurrency](../language/12-concurrency.md) for actor rules.

## Sequences

```ll
public protocol Sequence {
    associatedtype Element
    associatedtype Iterator: IteratorProtocol where Iterator.Element == Element
    func makeIterator() -> Iterator
}

public protocol IteratorProtocol {
    associatedtype Element
    mutating func next() -> Element?
}
```

## Panic and preconditions

```ll
public func fatalError(_ message: String) -> Never
public func precondition(_ condition: Bool, _ message: String = "")
public func assert(_ condition: Bool, _ message: String = "")
```

Debug builds evaluate `assert`; release builds strip it. `precondition` remains in release for programmer errors.

## Memory identity

```ll
public func withUnsafePointer<T, R>(
    to value: inout T,
    _ body: (Pointer<T>) throws(E) -> R
) throws(E) -> R
```

## Type metadata (reflection-lite)

```ll
public struct TypeName {
    public let qualified: String
    public static func of(_ type: (some Any).Type) -> TypeName
}
```

Full reflection is intentionally limited; prefer `Codable` and compile-time generics for serialization.

## Numeric literals helpers

Core re-exports literal conformances for integers, floats, `Bool`, and `String` so inference works in expressions (see [Types](../language/02-types.md)).

## See also

- [Collections](collections.md) — COW containers built on Core protocols
- [Memory](memory.md) — arenas for hot paths
