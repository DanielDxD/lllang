# Protocols and extensions

Protocols define requirements and may include **default implementations**. They are **generic** over type parameters listed in angle brackets — there is no separate `associatedtype`, `any`, or `some` syntax.

**Related:** [Generics](08-generics.md) · [Structs, classes, enums](06-structs-classes-enums.md)

## Protocol declarations

```ll
public protocol Container<Element> {
    var count: Int { get }
    func get(index: Int) -> Element?
    func append(item: Element) throws
}
```

Type parameters on protocols participate in conformance checking the same way as on structs:

```ll
public struct Stack<T>: Container<T> {
    private var storage: [T] = []
    public var count: Int => storage.count
    public func get(index: Int) -> T? {
        guard index >= 0 && index < count else { return nil }
        return storage[index]
    }
    public func append(item: T) { storage.append(item) }
}
```

## Default method bodies

Protocols may provide bodies directly — no extension required for defaults:

```ll
public protocol Summarizable {
    func sum() -> Double
    func average() -> Double {
        let c = count
        return c == 0 ? 0 : sum() / Double(c)
    }
    var count: Int { get }
}
```

Types conforming to `Summarizable` inherit `average()` unless they override it.

## Requirements vs defaults

| Kind | Syntax |
|------|--------|
| Property requirement | `var name: T { get }` or `{ get set }` |
| Method requirement | `func name(...) -> R` |
| Default implementation | Same signature with `{ ... }` body in protocol |
| Static requirements | `static func` / `static var` |

Requirements cannot have storage. Default implementations may call other protocol requirements.

## Constraining generics

```ll
public func merge<C: Container>(a: C, b: C) -> C where C.Element: Equatable {
    var result = a
    for i in 0..<b.count {
        if let item = b.get(index: i) { try? result.append(item: item) }
    }
    return result
}
```

## Protocol types as parameters

When a function accepts any conforming type without preserving the concrete `C`, use the protocol as a type:

```ll
public func printAll<C: Container>(c: C) where C.Element: CustomStringConvertible {
    for i in 0..<c.count {
        if let item = c.get(index: i) { print(item) }
    }
}
```

For heterogeneous storage, package-defined existential wrappers (e.g. `AnyContainer<Element>`) use witness tables — the surface syntax stays `Container<Element>` at compile time where possible.

## Class-only protocols

Mark class-bound protocols when reference semantics matter:

```ll
public protocol Listener: class {
    func onEvent()
}
```

Implementations must be classes (weak delegates, identity).

## Error protocol

```ll
public protocol Error: CustomStringConvertible {
    var message: String { get }
}
```

Enums and structs adopt `Error` for typed throws.

## Extensions

Extensions add members to any type in scope:

```ll
extension String {
    public func trimmed() -> String {
        // implementation
    }
}

extension Container where Element: Comparable {
    public func sorted() -> [Element] {
        var out: [Element] = []
        for i in 0..<count {
            if let e = get(index: i) { out.append(e) }
        }
        return out.sorted()
    }
}
```

Conditional extensions (`where` clauses) apply only when constraints hold.

### Protocol extensions

Additional defaults can live in extensions for organization:

```ll
extension Container where Element: Hashable {
    public func unique() -> [Element] { ... }
}
```

Same rules as defaults in the protocol body — no stored properties.

## Conformance visibility

Conformance must be visible where requirements are invoked. `public` types need `public` conformance methods when exported in `.sbl` stubs.

## Composition

Protocols compose by listing multiple conformances on types. Protocol inheritance lists parent protocols:

```ll
public protocol Identifiable<ID> {
    var id: ID { get }
}

public protocol Entity: Identifiable<UUID>, Codable {}
```

## Operator protocols

`Equatable`, `Comparable`, `Hashable`, and `Codable` are standard library protocols (see [Codable](17-codable.md)).

## Design guidelines

- Prefer small protocols with defaults over large “god” interfaces.
- Use primary constructors on structs rather than factory protocols when possible.
- Default implementations should be pure or document side effects.

## Summary

| Swift/C# concept | LLLang |
|------------------|--------|
| associatedtype | Generic protocol params `P<T>` |
| any / some | Not used; generics + existentials as needed |
| interface default methods | Defaults in protocol body |
| extension methods | `extension Type { }` |

Next: [Generics](08-generics.md) for functions and type parameters.
