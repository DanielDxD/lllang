# Statements

Statements perform effects: mutation, control transfer, and structured concurrency scopes. When a braced block appears in expression position, its **last** expression-valued statement becomes the block’s value; this chapter focuses on statement-only usage.

**Related:** [Expressions](04-expressions.md) · [Error handling](10-error-handling.md) · [Concurrency](12-concurrency.md)

## Blocks

```ll
{
    let x = 1
    doWork(x)
}
```

Local declarations shadow outer names. Blocks used as function bodies may `return` early.

## Expression statements

Any expression may stand alone as a statement when its value is discarded:

```ll
computeTotal()
items.append(item)
```

## Assignment and mutation

```ll
counter += 1
point.x = 10
self.status = .active
arr[i] = value
```

Compound assignment operators exist for arithmetic and bitwise ops. `let` bindings cannot be reassigned.

## Control flow: if

Statement form matches expression form but branches need not match types:

```ll
if user.isAdmin {
    grantAccess()
} else {
    deny()
}
```

`guard` ensures early exit:

```ll
func process(input: String?) throws(ParseError) {
    guard let text = input, !text.isEmpty else {
        throw .emptyInput
    }
    parse(text)
}
```

## Control flow: switch

Exhaustive `switch` on statements uses the same `pattern =>` arms without `break`:

```ll
switch event {
    .click(x, y) => handleClick(x, y),
    .key(.escape) => dismiss(),
    .key(code) where code.isPrintable => insert(code),
    _ => ignore(),
}
```

Each arm may be a single statement, a block, or multiple statements separated by newlines when wrapped in `{ }`.

## Loops

### while and do-while

```ll
while node != nil {
    visit(node!)
    node = node!.next
}

do {
    readChunk()
} while hasMore
```

Prefer optional binding over force-unwrap in new code.

### for-in

```ll
for item in collection { process(item) }
for (i, v) in items.enumerated() { print(i, v) }
for key in map.keys { ... }
for g in text.graphemes { ... }
```

Ranges:

```ll
for i in 0..<10 { sum += i }
```

## Jump statements

| Statement | Role |
|-----------|------|
| `return` | Exit function with optional value |
| `throw` | Raise typed error |
| `break` | Exit loop (not switch) |
| `continue` | Next loop iteration |
| `fallthrough` | Not used in switch (exhaustive arms) |

```ll
for x in items {
    if x.skip { continue }
    if x.done { break }
    handle(x)
}
```

## defer

`defer` runs at scope exit, LIFO order:

```ll
func withLock() throws {
    lock.acquire()
    defer { lock.release() }
    try criticalSection()
}
```

`defer` on async functions runs when the synchronous scope ends, not when the async work completes — use `using` patterns for async cleanup where needed.

## Error handling statements

`try` as a statement:

```ll
try save(document)
try? logOptional()
try! assertLoaded()  // traps
```

`do`/`catch` blocks for multiple statements:

```ll
do {
    try open()
    try process()
} catch (e: IoError) {
    recover(e)
}
```

Expression-level `catch` is preferred for single expressions (see [Error handling](10-error-handling.md)).

## async and await statements

```ll
async func loadAll() {
    let a = await fetchA()
    let b = await fetchB()
    merge(a, b)
}
```

`async let` for structured concurrency:

```ll
async let img = fetchImage(url)
async let meta = fetchMeta(url)
let page = Page(await img, await meta)
```

## unsafe statements

```ll
unsafe {
    let ptr = alloc(size)
    defer { free(ptr) }
    memcpy(dest, src, size)
}
```

See [Interop, FFI & unsafe](16-interop-ffi-unsafe.md).

## labeled statements

Labels disambiguate `break`/`continue` on nested loops:

```ll
outer: for row in matrix {
    for cell in row {
        if cell.isSentinel { break outer }
    }
}
```

## Empty statement

A lone `;` is an empty statement — rarely needed because newlines terminate statements.

## Pattern matching statements

`if case` / `switch` with destructuring bind names without extra `let` in patterns (see [Pattern matching](13-pattern-matching.md)):

```ll
if case .some(value) = optional { use(value) }
```

## Compiler-checked exhaustiveness

`switch` on enums and sealed hierarchies must cover all cases or include a wildcard with a `where` guard strategy documented by the type. The compiler warns on non-exhaustive switches when a default arm cannot prove coverage.

## Summary

| Topic | LLLang behavior |
|-------|-----------------|
| Switch | No `break`; `=>` arms |
| Semicolons | Optional |
| if/switch | Also expressions |
| guard | Early exit |
| defer | Scope exit cleanup |

Next: nominal types in [Structs, classes, enums](06-structs-classes-enums.md).
