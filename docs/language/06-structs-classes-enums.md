# Structs, classes, and enums

LLLang’s object model combines **value semantics** (structs, enums) with **reference semantics** (classes) under ARC. Primary constructors keep type definitions concise; dispatch defaults to static for performance.

**Related:** [Protocols & extensions](07-protocols-extensions.md) · [Memory model](11-memory-model.md) · [Pattern matching](13-pattern-matching.md)

## Structs

Structs are copied by default; large buffers use copy-on-write internally.

```ll
public struct Money(amount: Decimal, currency: String): Codable, Equatable {
    public func add(_ other: Money) throws(CurrencyError) -> Money {
        guard currency == other.currency else { throw .mismatch }
        Money(amount + other.amount, currency)
    }
}
```

Members can be `let` or `var`. The compiler allows mutating methods on `var` bindings without a `mutating` keyword.

### Primary constructor parameters

Parameters in the header become stored properties unless marked otherwise:

```ll
public struct Rect(x: Double, y: Double, width: Double, height: Double) {
    public var center: (x: Double, y: Double) => (x + width / 2, y + height / 2)
}
```

Default values and visibility modifiers on parameters are supported:

```ll
public struct Config(public let name: String, timeout: Duration = .seconds(30))
```

## Classes

Classes are heap-allocated reference types with deterministic `deinit`.

```ll
public class Order(public let id: UUID): Codable {
    public private(set) var items: [Item] = []
    public weak var customer: Customer?

    public var total: Decimal => items.sum((i) => i.lineTotal)

    public func add(item: Item) throws(OrderError) {
        guard item.qty > 0 else { throw .invalidQuantity }
        items.append(item)
    }

    deinit {
        Log.debug("Order \(id) released")
    }
}
```

### Inheritance and dispatch

| Modifier | Meaning |
|----------|---------|
| (none) | Non-virtual method — static dispatch |
| `virtual` | Overridable instance method |
| `override` | Replace base virtual method |
| `abstract` | No body; concrete subclasses must implement |
| `sealed` | No subclassing outside declaring package |

```ll
public sealed class ViewModel {
    public virtual func refresh() { load() }
}

public class ListViewModel: ViewModel {
    public override func refresh() {
        super.refresh()
        prefetch()
    }
}
```

Single inheritance for classes; protocol conformance is separate.

### Identity

```ll
if a === b { print("same object") }
```

## Enums

Enums are sum types **without** the `case` keyword:

```ll
public enum Shape {
    circle(radius: Double),
    rect(w: Double, h: Double),
    point,
}

public enum HttpStatus: Int {
    ok = 200,
    notFound = 404,
    serverError = 500,
}
```

Associated values and raw values may combine per variant where meaningful.

### Methods on enums

```ll
extension Shape {
    public func area() -> Double {
        switch self {
            .circle(r) => Double.pi * r * r,
            .rect(w, h) => w * h,
            .point => 0,
        }
    }
}
```

Patterns **bind by default** — `.circle(r)` not `.circle(let r)`.

## Protocol conformance on nominal types

List protocols after the base name:

```ll
public struct User(id: UUID, name: String): Codable, Equatable, Hashable
public class Repository: Disposable
```

The compiler synthesizes conformances when requirements match (e.g. `Equatable` for structs whose fields are equatable).

## Nested types

```ll
public class Parser {
    public enum Token { ident(String), number(Double), eof }
    public struct State { var pos: Int = 0 }
}
```

Nested types are accessed as `Parser.Token`.

## destructuring and factories

Static factory methods replace some initializer patterns:

```ll
public struct Angle {
    public static func degrees(_ d: Double) -> Angle { ... }
    public static func radians(_ r: Double) -> Angle { ... }
}
```

## Copying and assignment

- Struct assignment duplicates stored properties (COW for large fields).
- Class assignment copies the reference (ARC retain count +1).
- Enum assignment copies the active variant payload.

## Equatable and Hashable

Synthesized when all stored properties support it. Manual implementation:

```ll
public struct Key: Equatable {
    public let namespace: String
    public let name: String
    public static func ==(lhs: Key, rhs: Key) -> Bool {
        lhs.namespace == rhs.namespace && lhs.name == rhs.name
    }
}
```

## Codable

See [Codable](17-codable.md) for serialization details.

## Comparison table

| Feature | struct | class | enum |
|---------|--------|-------|------|
| Semantics | Value | Reference (ARC) | Value |
| Inheritance | No | Yes (single) | No |
| deinit | No | Yes | No |
| Primary ctor | Yes | Yes | N/A |
| Pattern matching | Tuple-like | `is` / casting | Primary |

## Common patterns

**State machine enum:**

```ll
enum Job {
    idle,
    running(progress: Double),
    failed(reason: String),
    done(result: Data),
}
```

**Class + struct split:** small immutable `struct` models on the wire; `class` for long-lived UI or graph nodes with `weak` back-links.

## Summary

- Construct with `Type(args)` — no `new`.
- Enums: no `case` keyword; patterns bind without extra `let`.
- Methods non-virtual unless marked `virtual`.
- Access: `open` / `public` / `internal` / `fileprivate` / `private` (no `protected`; see ADR 009).

Continue with [Protocols & extensions](07-protocols-extensions.md).
