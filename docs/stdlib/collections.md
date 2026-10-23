# Std.Collections

**Std.Collections** provides resizable arrays, hash maps, sets, and copy-on-write (COW) buffer types used by `String` and binary payloads.

```ll
import Std.Collections
```

**MVP (Phase 1.2):** `[T]` / `[Key: Value]` builtins with `count`, index, `append` / insert, and `[String].joined(sep)`. **`[Int]` HOFs** ship: `map` / `filter` / `reduce` / `sorted` / `any` / `all` (closures with scalar captures). **Set** algebra: `union` / `intersection` / `difference`. Opaque `some Sequence<…>` parameters remain Later.

## Array

Type syntax `[T]` with literal `[1, 2, 3,]`. Empty literals need an explicit type annotation (`let xs: [Int] = []`).

```ll
public struct Array<Element> {
    public init()
    public init(repeating: Element, count: Int)
    public var count: Int { get }
    public var isEmpty: Bool { get }
    public subscript(index: Int) -> Element { get set }
    public mutating func append(_ element: Element)
    public mutating func reserveCapacity(_ minimum: Int)
}
```

COW: assignment shares storage until a mutating write on a non-unique buffer triggers copy.

String arrays ship `joined(separator:)` as an MVP helper.

## Dictionary

Type syntax `[Key: Value]` (MVP keys/values are typically `String`).

```ll
public struct Dictionary<Key: Hashable, Value> {
    public init()
    public var count: Int { get }
    public subscript(key: Key) -> Value? { get set }
    public mutating func removeValue(forKey key: Key) -> Value?
}
```

## Set

```ll
public struct Set<Element: Hashable> {
    public init()
    public var count: Int { get }
    public func contains(_ element: Element) -> Bool
    public mutating func insert(_ element: Element)
}
```

Set literals and algebra (`union` / `intersection` / …) are **Later**.

## COW buffer

Low-level shared storage for stdlib internals and advanced users:

```ll
public struct COWBuffer {
    public init(capacity: Int = 0)
    public var count: Int { get }
    public mutating func append(_ byte: UInt8)
    public mutating func makeUnique(copyIfNeeded: Bool = true)
}
```

`makeUnique` forces a private copy before in-place mutation across aliases.

## Algorithms

`sort` / `binarySearch` / `grouped` over generic sequences are **Later**. Prefer explicit loops for now.

## Equality and hashing

Collections conform to `Equatable` and `Hashable` when their element types do (MVP coverage varies by element kind).

## See also

- [Memory model](../language/11-memory-model.md)
- [Json](json.md) — streaming array encoding
