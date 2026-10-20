# Types

LLLang is **statically typed** with **local type inference** only: the compiler infers types for locals, fields with initializers, and obvious literals, but never for module-level bindings without explicit annotations.

**Related:** [Optionals](09-optionals.md) · [Generics](08-generics.md) · [Structs, classes, enums](06-structs-classes-enums.md)

## Type categories

| Category | Examples | Semantics |
|----------|----------|-----------|
| Value types | `struct`, `enum`, tuples | Copied; COW when large |
| Reference types | `class` | ARC heap objects |
| Protocol types | `Equatable`, `Container<T>` | Constraints & existential use |
| Function types | `(Int, String) -> Bool` | First-class |
| Special | `Optional<T>`, `Result<T,E>`, `Never` | Built-in generics |

There is no `object` or `dynamic` — use generics and protocols.

## Built-in numeric types

Signed integers: `Int` (platform word), `Int8` … `Int64`.  
Unsigned: `UInt`, `UInt8` … `UInt64`.  
Floats: `Float32`, `Float64`.  
Decimal: `Decimal` for money and exact base-10 arithmetic.

### Conversions

**Widening** conversions are implicit when no precision is lost:

```ll
let x: Int32 = 10
let y: Int64 = x        // OK: widening
let z: Float64 = y      // OK
```

**Narrowing** requires an explicit cast:

```ll
let big: Int64 = 1_000_000
let small = Int32(big)  // explicit narrowing; may trap or throw per overflow mode
```

Literals adapt to the expected type in local context:

```ll
let a: UInt8 = 255
let b = 1_000_000       // inferred Int
```

## Boolean and unit

`Bool` has `true` and `false`. Logical operators short-circuit.

`Void` (alias `()`) is the type of statements and `func` with no meaningful return. Functions omit `-> Void` when they return nothing:

```ll
func log(msg: String) {
    print(msg)
}
```

## Strings and views

`String` is an immutable UTF-8 buffer. Indexing uses **views**, not opaque indices:

```ll
let s = "café"
for g in s.graphemes { print(g) }
for c in s.chars { print(c.codePoint) }
for b in s.bytes { print(b) }
```

Substrings are typically `String` slices or `Substring` depending on API; all slicing uses integer offsets or view iterators, never `String.Index`.

## Arrays, dictionaries, sets

```ll
let nums: [Int] = [1, 2, 3,]
let map: [String: Int] = ["a": 1, "b": 2,]
let set: Set<UUID> = [id1, id2]
```

Trailing commas are allowed. Empty collection types need context or annotation:

```ll
let empty: [String] = []
```

## Tuples

**Named tuples** are first-class types:

```ll
let point = (x: 1.0, y: 2.0)
let (x, y) = point
let dx = point.x

type Point2 = (x: Double, y: Double)
```

Destructuring works in assignments, `for`, and patterns (see [Pattern matching](13-pattern-matching.md)).

## Optional

See [Optionals](09-optionals.md). Syntax sugar: `T?` means `Optional<T>`.

## Functions as types

```ll
type Predicate<T> = (T) -> Bool
type Handler = (Int, String) throws(NetworkError) -> Void

let isEven: Predicate<Int> = (n) => n % 2 == 0
```

`async` functions have async function types in metadata; at call sites they behave like `() -> T` with implicit task scheduling (see [Concurrency](12-concurrency.md)).

## Protocol and class types

A variable typed as a **protocol** holds a value or reference that conforms. Protocols are **generic** at definition:

```ll
protocol Container<Element> {
    func get(index: Int) -> Element?
}

func first<C: Container>(c: C) -> C.Element? where C.Element: Equatable {
    c.get(index: 0)
}
```

There is no `any Container<Int>` keyword soup — use `Container<Int>` as a type name when you need existential storage; the compiler lowers to witness tables.

## Type aliases

```ll
type UserId = UUID
type Matrix = [[Float64]]
public type JsonValue = Optional<AnyCodable>  // illustrative
```

Aliases are transparent for overload resolution and do not create new nominal types.

## Inference rules (local only)

Inference applies when:

1. A `let`/`var` has an initializer and no explicit type.
2. A primary constructor parameter has a default or literal initializer in the struct header.
3. Generic arguments can be inferred from parameters or return at the call site.

Inference does **not** apply to:

- Module-level `let`/`var` without types
- Public API surface in `.sbl` exports (must be explicit)
- Recursive references without a fixed point annotation

```ll
// OK locally
let items = [1, 2, 3]           // [Int]
let first = items[0]            // Int

// Required at module scope
public let DefaultTimeout: Duration = .seconds(30)
```

## Metatype types

`TypeName` in expression position refers to the type itself for construction and static members:

```ll
let t = Int.self
let instance = Order(id)   // no `new`
```

## Availability and platform types

Platform-specific types (`Platform.FileHandle`) live in packages; the core language types are platform-agnostic. `@available(platform, version)` gates declarations (see [Attributes](14-attributes.md)).

## Type equality and identity

- Value types: structural equality only when `Equatable` is implemented.
- Reference types: identity via `===` / `!==` (same ARC object).
- Protocol constraints: nominal conformance checked at compile time.

## Summary

| Topic | LLLang choice |
|-------|----------------|
| Inference | Local only |
| Null | Not present; use `Optional` |
| Numeric conversions | Widening implicit, narrowing explicit |
| String indexing | Views (`chars`, `bytes`, `graphemes`) |
| Generics | On types, functions, protocols |

Continue with [Declarations](03-declarations.md) for variables, functions, and visibility.
