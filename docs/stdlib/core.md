# Std.Core

**Std.Core** is implicit: every compilation unit sees its types and protocols without an import. It defines language-adjacent primitives, universal protocols, and small utilities used across the stdlib.

**MVP honesty:** Phase 1.1 ships concrete builtins (`Optional`, `Result`, `Equatable`, `Sendable`, …). Opaque `some` / existential `any` / `associatedtype` sugars are **Later** — do not treat sketches that use them as compile-ready APIs.

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
    // MVP: combine concrete Hashable values; `some Hashable` sugar is Later
    public mutating func combine(_ value: Int)
    public mutating func combine(_ value: String)
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

`Identifiable` with `associatedtype ID` is a **design target (Later)**. Today, prefer an explicit `id: String` / `id: Int` field (see `examples/identity`).

```ll
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

`Sequence` / `IteratorProtocol` with `associatedtype` are **Later** protocol sketches. Phase 1.1 iteration is over concrete arrays, ranges, and dictionaries — not a generic `Sequence` protocol.

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
    // MVP: pass a concrete metatype; `(some Any).Type` sugar is Later
    public static func of(_ type: String.Type) -> TypeName
}
```

Full reflection is intentionally limited; prefer `Codable` and compile-time generics for serialization.

## Numeric literals helpers

Core re-exports literal conformances for integers, floats, `Bool`, and `String` so inference works in expressions (see [Types](../language/02-types.md)).

## See also

- [Collections](collections.md) — COW containers built on Core protocols
- [Memory](memory.md) — arenas for hot paths
