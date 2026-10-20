# Std.Collections

**Std.Collections** provides resizable arrays, hash maps, sets, and copy-on-write (COW) buffer types used by `String` and binary payloads.

```ll
import Std.Collections
```

## Array

Type syntax `[T]` with literal `[1, 2, 3,]`.

```ll
public struct Array<Element> {
    public init()
    public init(repeating: Element, count: Int)
    public var count: Int { get }
    public var isEmpty: Bool { get }
    public subscript(index: Int) -> Element { get set }
    public mutating func append(_ element: Element)
    public mutating func reserveCapacity(_ minimum: Int)
    public func map<U>(_ transform: (Element) throws(E) -> U) throws(E) -> [U]
}
```

COW: assignment shares storage until a mutating write on a non-unique buffer triggers copy.

## Dictionary

Type syntax `[Key: Value]`.

```ll
public struct Dictionary<Key: Hashable, Value> {
    public init()
    public init(uniqueKeysWithValues: [(Key, Value)]) throws(DictionaryError)
    public var count: Int { get }
    public subscript(key: Key) -> Value? { get set }
    public mutating func removeValue(forKey key: Key) -> Value?
    public func mapValues<U>(_ transform: (Value) throws(E) -> U) throws(E) -> [Key: U]
}
```

## Set

```ll
public struct Set<Element: Hashable> {
    public init()
    public init(_ elements: some Sequence<Element>)
    public var count: Int { get }
    public func contains(_ element: Element) -> Bool
    public mutating func insert(_ element: Element) -> (inserted: Bool, memberAfterInsert: Element)
    public mutating func formUnion(_ other: Set<Element>)
}
```

Set literals use `[a, b, c]` when element type is inferable from context.

## COW buffer

Low-level shared storage for stdlib internals and advanced users:

```ll
public struct COWBuffer {
    public init(capacity: Int = 0)
    public var count: Int { get }
    public mutating func append(_ byte: UInt8)
    public func withUnsafeBytes<R>(_ body: (Span<UInt8>) throws(E) -> R) throws(E) -> R
    public mutating func makeUnique(copyIfNeeded: Bool = true)
}
```

`makeUnique` forces a private copy before in-place mutation across aliases.

## Algorithms

```ll
public func sort<T: Comparable>(_ array: inout [T])
public func binarySearch<T: Comparable>(_ array: [T], _ value: T) -> Int?
public func grouped<T, K: Hashable>(
    _ sequence: some Sequence<T>,
    by key: (T) -> K
) -> [K: [T]]
```

## Equality and hashing

Collections conform to `Equatable` and `Hashable` when their element types do.

## See also

- [Memory model](../language/11-memory-model.md)
- [Json](json.md) — streaming array encoding
