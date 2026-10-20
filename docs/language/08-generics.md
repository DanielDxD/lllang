# Generics

Generics let functions and types parametrize over types while preserving static dispatch and type safety. LLLang keeps inference **local** — generic parameters must be deducible at the call site or specified explicitly.

**Related:** [Protocols & extensions](07-protocols-extensions.md) · [Types](02-types.md)

## Generic functions

```ll
public func swap<T>(_ a: inout T, _ b: inout T) {
    let tmp = a
    a = b
    b = tmp
}

public func firstIndex<T: Equatable>(of value: T, in array: [T]) -> Int? {
    for (i, v) in array.enumerated() {
        if v == value { return i }
    }
    return nil
}
```

Multiple type parameters:

```ll
public func zip<A, B>( _ xs: [A], _ ys: [B]) -> [(A, B)] {
    var out: [(A, B)] = []
    let n = min(xs.count, ys.count)
    for i in 0..<n { out.append((xs[i], ys[i])) }
    return out
}
```

## Generic types

```ll
public struct Box<T> {
    public var value: T
    public init(value: T) { self.value = value }
}

public class Node<T> {
    public var value: T
    public var next: Node<T>?
    public init(value: T) { self.value = value }
}
```

Enums and protocols are generic too:

```ll
public enum Result<T, E: Error> {
    ok(T),
    err(E),
}

public protocol Map<Key, Value> {
    func get(key: Key) -> Value?
    func set(key: Key, value: Value)
}
```

## Constraints

Constraints list protocol conformances and type relationships:

```ll
public func clone<C: Cloneable>(c: C) -> C where C: Copyable {
    c.clone()
}
```

### Same-type requirements

```ll
public func combine<T, U>(t: T, u: U) -> (T, U) where T == U {
    (t, u)
}
```

### Class constraints

```ll
public func attach<L: Listener>(listener: L) where L: class {
    registry.add(listener)
}
```

## Where clauses

Long constraint lists attach with `where` after the signature:

```ll
public func sortInPlace<T, C: Container<T>>(c: inout C)
    where T: Comparable, C.Element == T
{
    // specialized sort using Container API
}
```

## Inference at call sites

```ll
let b = Box(42)              // Box<Int>
let keys = firstIndex(of: "a", in: ["a", "b"])  // Int?

swap(&x, &y)                 // T from arguments
```

When inference fails:

```ll
let box = Box<String>("hi")
let empty: [Int] = []
let first = firstIndex(of: 1, in: empty)
```

## Specialization and codegen

The compiler monomorphizes hot generic paths and may share code for reference-type arguments. Public generic APIs in libraries export mangled symbols in `.bll` with `.sbl` signatures for cross-package linking.

## Generic protocols (review)

Protocols carry type parameters:

```ll
public protocol Iterator<Element> {
    mutating func next() -> Element?
}
```

Conformance `struct RangeIterator: Iterator<Int>` supplies `Element = Int`.

## Variance

- Generic struct/enums: invariant in parameters.
- Function types: parameters contravariant, results covariant in formal typing for protocol subtyping checks.
- Practical rule: treat generics as invariant unless documentation states otherwise for read-only protocols.

## Type erasure (advanced)

When heterogeneous collections of generic conformers are required, the library provides typed erasers:

```ll
public struct AnyContainer<Element>: Container<Element> {
    // witness forwarding
}
```

Prefer concrete generics at API boundaries; use erasure at integration seams.

## Generic extensions

```ll
extension Array where Element: Numeric {
    public func total() -> Element {
        reduce(Element.zero, +)
    }
}
```

## Recursive constraints

```ll
public protocol TreeNode<T: TreeNode<T>> {
    var children: [T] { get }
}
```

F-bounded patterns are allowed with caution — the compiler detects ill-formed cycles.

## Default type parameters

```ll
public struct Dict<K, V, H: Hasher<K> = DefaultHasher<K>> {
    // ...
}
```

Defaults appear in generic parameter lists with `=`.

## Diagnostics

Common errors:

- Could not infer `T` — add annotation or explicit generic args.
- `T` does not conform to `Equatable` — add constraint.
- Conflicting requirements in `where` — simplify signature.

## Summary

| Topic | Behavior |
|-------|----------|
| Protocol generics | `protocol P<T>` |
| Inference | Call-site only |
| Constraints | `: Protocol`, `where` |
| Keywords | No `associatedtype` |

Next: [Optionals](09-optionals.md) for nullable types without null.
