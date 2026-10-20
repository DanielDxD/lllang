# Optionals

LLLang replaces null references with **`Optional<T>`** (spelled `T?`). Absence is always explicit in the type system; `nil` is the only way to denote missing values.

**Related:** [Types](02-types.md) · [Expressions](04-expressions.md) · [Pattern matching](13-pattern-matching.md)

## Optional types

```ll
let name: String? = nil
var user: User? = findUser(id)
```

`Optional` is a standard enum:

```ll
enum Optional<T> {
    none,
    some(T),
}
```

Sugar: `T?` desugars to `Optional<T>`.

## Creating optionals

```ll
let a: Int? = 42        // implicit lift to .some(42)
let b: Int? = nil
let c = Optional.some(1)
let d = Optional<Int>.none
```

Non-optional values do not implicitly become optional except in assignment to `T?` or parameter passing with lift.

## Unwrapping

### if let / guard let

```ll
if let user = optionalUser {
    greet(user.name)
}

guard let config = loadConfig() else {
    return .defaults
}
```

### Pattern matching

Patterns bind without extra `let`:

```ll
switch optionalUser {
    .some(u) => print(u.name),
    .none => print("anonymous"),
}

// or shorthand when enum case inferred
switch optionalUser {
    some(u) => print(u.name),
    none => print("anonymous"),
}
```

### Force unwrap

`!` traps if `none`:

```ll
let always = optionalUser!   // avoid in production paths
```

Prefer `guard let` or `??`.

## Optional chaining

```ll
let city = user?.address?.city
let upper = user?.name?.uppercased()
let count = items?[0]?.count
```

Chaining short-circuits on first `none`.

## Nil-coalescing

```ll
let display = nickname ?? legalName ?? "Guest"
let level = settings?.level ?? .default
```

Right-hand side evaluates lazily when implemented via built-in (standard library guarantees lazy for `??`).

## Optional map and flatMap

Standard library helpers:

```ll
let len = name.map((s) => s.count)
let domain = user.flatMap((u) => u.email?.domain)
```

Prefer these over nested `if let` when transforming.

## Comparison

Optionals of `Equatable` types compare when both sides are `some` or both `none`:

```ll
if name == nil { ... }
if optionalA == optionalB { ... }
```

## Optionals and collections

There is no `null` in JSON decoding — missing keys produce `none` for `T?` fields (see [Codable](17-codable.md)).

```ll
public struct Profile: Codable {
    public let bio: String?   // omitted key -> nil
}
```

## Optionals with numeric and value types

Value types inside optionals copy on unwrap:

```ll
var opt = Money(1, "USD")
if var m = opt {
    m.amount = 2
    opt = m
}
```

## Weak references

`weak var x: T?` for classes — automatically `none` when target deallocates (see [Memory model](11-memory-model.md)). Not the same as optional value semantics but uses `T?` storage.

## Anti-patterns

| Avoid | Prefer |
|-------|--------|
| `value!` chains | Chaining and `??` |
| `Optional<T>` everywhere | Defaults, empty collections |
| Sentinel `-1` for missing | `Int?` |

## Interop

C pointers that may be null import as `Pointer<T>?` or `Optional<UnsafePointer<T>>` in unsafe blocks — never as plain references.

## Summary

- No `null` keyword — only `nil` and `T?`.
- Unwrap with `if let`, `guard`, patterns, or operators.
- Chaining: `?.` and `??`.

Next: [Error handling](10-error-handling.md) for failures that are not optional absence.
