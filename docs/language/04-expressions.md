# Expressions

Almost every LLLang construct that produces a value is an **expression**, including `if`, `switch`, and `try` with trailing `catch`. Expressions compose with predictable precedence (see [operators](../spec/operators.md)).

**Related:** [Statements](05-statements.md) · [Pattern matching](13-pattern-matching.md) · [Error handling](10-error-handling.md)

## Literals and identifiers

Literals are described in [Lexical structure](01-lexical-structure.md). Identifiers resolve to locals, members, or static members via nominal typing.

## Operators

Arithmetic, comparison, logical, and bitwise operators behave as in C-family languages. Additional forms:

| Operator | Meaning |
|----------|---------|
| `??` | Nil-coalescing |
| `?.` `?[]` | Optional chaining |
| `..<` `...` | Half-open and closed ranges |
| `is` | Type check with optional binding |
| `as` | Cast (explicit narrowing) |

```ll
let len = user?.profile?.name?.count ?? 0
let slice = items[1..<4]
if shape is Circle c {
    print(c.radius)
}
```

Assignment is a statement, not an expression (no chained assignment).

## Function and method calls

Calls are positional by default:

```ll
move(10, 20)
move(dx: 10, dy: 20)   // optional names
```

**Method chaining** is intended for **query-style** APIs (fluent builders, iterators), not SQL-like query syntax:

```ll
let names = users
    .filter((u) => u.active)
    .map((u) => u.name)
    .sorted()
```

There is no LINQ `from x in xs` comprehension form.

## Construction

```ll
let m = Money(19.99d, "USD")
let order = Order(UUID.v4())
```

Generic construction may require explicit type arguments when inference fails:

```ll
let box = Box<Int>(42)
```

## Member access

```ll
let c = point.x
let fn = Type.staticMethod
Type.staticMethod()
```

## Closures (lambdas)

```ll
let square = (n: Int) => n * n
let sum = items.reduce(0, (acc, x) => acc + x)
```

Trailing closure syntax when the last argument is a function:

```ll
items.sort { $0.name < $1.name }
```

Capture is by reference for `var` outer locals and by value for `let` unless `@escaping` rules apply on stored closures.

## If expressions

`if` produces a value when both branches do:

```ll
let label = if count == 0 { "empty" } else { "\(count) items" }

let max = if a > b { a } else { b }
```

`else if` chains are supported. Condition may bind with `is`:

```ll
let area = if shape is Circle c { Double.pi * c.radius * c.radius } else { 0 }
```

## Switch expressions

Exhaustive `switch` is an expression; arms use `pattern => expr`:

```ll
let kind = switch code {
    200 => "ok",
    404 => "missing",
    500...599 => "server",
    c where c >= 400 => "client",
    _ => "other",
}
```

No `break` — each arm is a single expression or block whose last value is taken. See [Pattern matching](13-pattern-matching.md).

## Try expressions

```ll
let data = try readFile(path)
let maybe = try? decode(input)
let forced = try! loadCert()   // trap on failure; use sparingly
```

**Expression catch** supplies a fallback:

```ll
let shipping = computeShipping(order) catch (e) => Money(0, "USD")
let msg = fetchLabel() catch (_: NetworkError) => "offline"
```

Typed catch clauses must match the function’s `throws(E)` type.

## Optional operations

```ll
if let user = optionalUser {
    greet(user.name)
}

guard let id = params["id"] else { return }

let name = user?.name
let upper = user?.name?.uppercased()
```

## String interpolation

```ll
"\(user.name) has \(count) items"
"\(price, .fixed(2))"
"\(date, .iso8601)"
```

Format descriptors are extensible via `TextFormat` in the standard library.

## Tuple and destructuring expressions

```ll
let (x, y) = (1, 2)
let (x: a, y: b) = point
swap(&a, &b)
```

## Collection and range expressions

```ll
[1, 2, 3,]
["k": v,]
1...10
0..<length
```

## Type expressions

`T.self`, generic instantiation `Map<String, Int>`, and protocol-constrained types appear in casts and annotations:

```ll
let anyEquatable: Equatable = 42  // when existential slot exists
```

## Await

Inside `async` functions:

```ll
let data = await client.get(url)
await MainActor.run { ui.refresh() }
```

## Operator methods

User-defined types implement operators via static methods or protocol hooks:

```ll
public struct Vector {
    public static func +(lhs: Vector, rhs: Vector) -> Vector { ... }
}
```

## Precedence and associativity

Unary `!`, `-`, `~` bind tightly. `is`/`as` sit between equality and multiplicative. `??` is low, above assignment. When in doubt, parenthesize.

## Pureness and effects

Expressions may throw, suspend (`await`), or perform I/O. The compiler does not enforce purity; `pure` is a documentation attribute for LLM and static analysis hints.

## Summary

| Construct | Expression? |
|-----------|-------------|
| `if` / `switch` | Yes, with unified type |
| `try` + `catch` | Yes |
| Statement blocks `{ }` | Last statement value if no early return |
| Assignment | No |

Continue to [Statements](05-statements.md) for control flow that does not produce values.
