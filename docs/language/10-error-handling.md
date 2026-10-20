# Error handling

LLLang uses **typed throws** so callers know which error types a function may propagate. Recoverable failures are not mixed with optional absence — use `throws(E)` and `Result` when both outcomes carry data.

**Related:** [Expressions](04-expressions.md) · [Statements](05-statements.md) · [Optionals](09-optionals.md)

## Error types

Any type conforming to `Error` may be thrown. Enums are idiomatic:

```ll
public enum OrderError: Error {
    invalidQuantity,
    outOfStock(sku: String),
    paymentDeclined(code: Int),
}

public struct ValidationError: Error {
    public let field: String
    public let message: String
}
```

## Declaring throwing functions

```ll
public func add(item: Item) throws(OrderError) {
    guard item.quantity > 0 else { throw .invalidQuantity }
    guard inventory.available(item.sku) else {
        throw .outOfStock(sku: item.sku)
    }
    items.append(item)
}
```

Multiple error types use a union type or a wrapper enum at API boundaries:

```ll
public enum ServiceError: Error {
    order(OrderError),
    network(NetworkError),
}

public func checkout() throws(ServiceError) { ... }
```

Untyped `throws` is allowed only for `throws(Error)` in low-level or scripting APIs — libraries should prefer concrete types.

## Throwing expression

```ll
throw OrderError.invalidQuantity
throw .outOfStock(sku: sku)   // when context types the error
```

## Propagation with try

```ll
public func process() throws(OrderError) {
    try validate()
    try persist()
}
```

`try` is required on calls that can throw within non-throwing functions.

## try? and try!

```ll
let maybe = try? decode(input)        // Optional<T>
let value = try! loadTrustedBundle()  // trap on error
```

`try!` is for invariant violations and tests — not user input.

## Expression catch

Single-expression recovery without a full `do` block:

```ll
let shipping = computeShipping(order) catch (e) => Money(0, "USD")

let label = fetchLabel() catch (e: NetworkError) {
    Log.warn("\(e)")
    "offline"
}
```

Catch parameter types must be compatible with the callee’s error type. Use `_` to ignore:

```ll
let x = risky() catch (_) => defaultValue
```

## do / catch statements

```ll
do {
    try openConnection()
    try sync()
} catch (e: NetworkError) {
    retry(e)
} catch (e: OrderError) {
    showOrderFailure(e)
}
```

Catch clauses are checked in order; the first match wins. Exhaustiveness is required when the thrown type is a closed enum and the function is public API (compiler warning if a case is missing).

## Result type

```ll
public enum Result<T, E: Error> {
    ok(T),
    err(E),
}

public func parse(input: String) -> Result<Data, ParseError> {
    if input.isEmpty { return .err(.empty) }
    return .ok(Data(input))
}
```

Bridge between throwing and functional style:

```ll
let r = Result(catching: { try decode(json) })  // std helper
```

## defer and errors

```ll
func write(path: Path) throws(IoError) {
    let handle = try File.open(path)
    defer { handle.close() }
    try handle.write(data)
}
```

If `close()` throws, suppressed errors follow language rules (first error wins; secondary logged in debug).

## Async throws

```ll
public func fetchOrder(id: UUID) async throws(HttpError) -> Order {
    let data = try await client.get("/orders/\(id)")
    try JSON.decode(data)
}
```

`await` applies before `try` on the same expression: `try await foo()`.

## Panics vs errors

| Mechanism | Use |
|-----------|-----|
| `throw` | Recoverable, typed |
| `fatalError("msg")` | Programming bug |
| `precondition` / `assert` | Debug vs release |
| Force unwrap `!` | Logic error on none |
| `try!` | Trusted paths only |

## Error equality and logging

Errors conform to `CustomStringConvertible` for logging. Do not rely on equality for control flow unless documented (e.g. enum cases).

## Interop

FFI boundaries map errno / HRESULT into Swift-like errors at the import layer; raw codes stay in `unsafe` modules.

## Testing errors

```ll
@Test
func rejectsZeroQuantity() {
    expectThrows(OrderError.invalidQuantity) {
        try order.add(item: Item(qty: 0))
    }
}
```

## Summary

| Form | Role |
|------|------|
| `throws(E)` | Typed propagation |
| `try` / `try?` / `try!` | Call throwing code |
| `catch (e) =>` | Expression fallback |
| `do` / `catch` | Multi-statement recovery |
| `Result` | Explicit success/failure values |

Next: [Memory model](11-memory-model.md) for ARC, weak, and deinit.
