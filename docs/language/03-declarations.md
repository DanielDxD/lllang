# Declarations

Declarations introduce names into scopes: variables, functions, types, and namespaces. LLLang uses **primary constructors** for many types and keeps parameter lists **positional by default** — callers may use names for clarity, but definitions do not use Swift-style external labels.

**Related:** [Lexical structure](01-lexical-structure.md) · [Structs, classes, enums](06-structs-classes-enums.md) · [Namespaces & modules](15-namespaces-modules.md)

## Access control

See [ADR 009](../design/adr/009-access-control.md). Swift-like set:

| Modifier | Meaning |
|----------|---------|
| `internal` | Visible within the same package (default) |
| `public` | Visible to importers of the package |
| `open` | Like `public`, and overridable/subclassable across packages |
| `fileprivate` | Visible only within the same source file |
| `private` | Visible only in the enclosing declaration |

There is **no** `protected` (use `fileprivate` or `public` instead).

Property visibility can split getter and setter:

```ll
public class Account {
    public private(set) var balance: Decimal = 0
    fileprivate var ledger: [Entry] = []
    private var pinHash: Data
}
```

## Variables and constants

`let` is immutable after initialization; `var` is mutable. Both are **properties** when declared on types (stored or computed).

```ll
let maxRetries = 3
var counter = 0

public struct Config {
    public let apiKey: String
    public var timeout: Duration = .seconds(10)
}
```

Module-level bindings require explicit types unless inside a function:

```ll
public let Version: String = "1.0.0"
```

### Computed properties

Shorthand read-only:

```ll
public var area: Double => width * height
```

Full accessors:

```ll
public var temperature: Double {
    get => _celsius
    set { _celsius = newValue }
}
```

Property observers:

```ll
public var status: Status {
    didSet { Log.info("status \(oldValue) -> \(status)") }
    willSet { prepare(for: newValue) }
}
```

The compiler infers mutating vs non-mutating for struct members; there is **no** `mutating` keyword.

## Functions and methods

```ll
func greet(name: String) -> String {
    "Hello, \(name)"
}

public func fetch(url: URL) async throws(HttpError) -> Data {
    try await HttpClient.shared.get(url)
}
```

- **Parameters** are positional at the call site: `greet("Ada")`. Named calls are optional: `greet(name: "Ada")`.
- **Typed throws:** `throws(OrderError)` not bare `throws` when the error type matters.
- **Async:** `async func` — no `Async` suffix on names; tasks are created implicitly at call boundaries.

Methods on types default to **non-virtual** dispatch. Use `virtual`, `override`, `abstract`, and `sealed` for class hierarchies (see [Structs, classes, enums](06-structs-classes-enums.md)).

```ll
public class Base {
    public func ping() { print("base") }           // non-virtual
    public virtual func pong() { print("base pong") }
}

public class Derived: Base {
    public override func pong() { print("derived") }
}
```

Static and type methods:

```ll
public struct Id {
    public static func generate() -> UUID { UUID.v4() }
}
```

## Initializers and construction

There is **no** `new` keyword. Construction is `TypeName(arguments)`.

Primary constructors appear in the type header:

```ll
public struct Money(amount: Decimal, currency: String): Codable

public class Order(public let id: UUID, created: Date = Date.now): Codable {
    // additional members
}
```

Secondary initializers use `init`:

```ll
public struct Money {
    public init(amount: Decimal, currency: String) { ... }
    public init?(json: String) { ... }  // failable
}
```

## Type declarations

```ll
public struct Point(x: Double, y: Double): Equatable

public enum Status { pending, active, archived }

public protocol Repository<Item> {
    func save(item: Item) throws(StorageError)
}

public class Service: Repository<Order> { ... }
```

Enums omit the `case` keyword (see [Structs, classes, enums](06-structs-classes-enums.md)).

## Delegates and function values

Named function types can be declared with `type` or assigned to `let`:

```ll
public type Comparator<T> = (T, T) -> Int

public var sortFn: Comparator<String> = (a, b) => a.compare(b)
```

There is no separate `delegate` keyword — use function types and closures.

## Namespaces and imports

```ll
namespace Shop.Billing

import Std.Json
import OtherPkg.Models as Models

public func run() { ... }
```

Types in the **same package** and namespace are visible without import. Cross-package access requires `import` (see [Namespaces & modules](15-namespaces-modules.md)).

## Extension declarations

Extensions add members to existing types in any file of the package:

```ll
extension Money {
    public func format() -> String => "\(amount, .fixed(2)) \(currency)"
}
```

See [Protocols & extensions](07-protocols-extensions.md).

## Attributes on declarations

```ll
@MainActor
public class ViewModel { ... }

@deprecated("Use fetchV2")
public func fetch() { ... }
```

See [Attributes](14-attributes.md).

## Overloading

Functions and initializers overload on parameter types. Return types alone cannot distinguish overloads. Generic functions overload with constraints.

## Declaration order

Forward references are allowed for functions and mutually recursive types when the compiler can stitch fixed types. Opaque recursive structs need indirection (`Box<T>` or class wrapper).

## Summary

| Feature | Syntax |
|---------|--------|
| Immutability | `let` |
| Positional params | Default; names optional at call |
| Construction | `Type(args)` |
| Visibility | `open`, `public`, `internal`, `fileprivate`, `private` (no `protected`; ADR 009) |
| Computed property | `=>` or `get`/`set` (+ `willSet`/`didSet`) |
| Async | `async func` |
| Errors | `throws(SpecificError)` |

Custom `subscript` and `operator` declarations are **not** supported yet (LL3001 → Later).

Next: [Expressions](04-expressions.md) for operators, calls, and literals in expression positions.
