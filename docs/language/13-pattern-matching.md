# Pattern matching

Patterns destructure values in `switch`, `if`, loops, and assignments. LLLang **binds names by default** in enum and tuple patterns — you write `.circle(r)` rather than `.circle(let r)`.

**Related:** [Expressions](04-expressions.md) · [Optionals](09-optionals.md) · [Structs, classes, enums](06-structs-classes-enums.md)

## Switch expressions and statements

Arms use `pattern => body`:

```ll
let area = switch shape {
    .circle(r) => Double.pi * r * r,
    .rect(w, h) => w * h,
    .point => 0,
}
```

Switch must be **exhaustive** for enums and sealed types. A wildcard `_` completes open ranges:

```ll
switch statusCode {
    200 => "ok",
    301...399 => "redirect",
    400...499 => "client",
    500...599 => "server",
    _ => "unknown",
}
```

No `break` between arms — each arm is independent.

## where guards

```ll
switch request {
    .get(path) where path.startsWith("/admin") => deny(),
    .get(path) => serve(path),
    .post(path, body) where body.count < max => accept(body),
    .post(_, _) => reject(.payloadTooLarge),
}
```

Guards may refer to bound names from the pattern.

## Enum patterns

Enums omit the `case` keyword in declarations; patterns use dot syntax:

```ll
enum Message {
    text(String),
    image(data: Data, mime: String),
    ping,
}

switch msg {
    .text(s) => print(s),
    .image(data, mime) where mime == "image/png" => renderPng(data),
    .image(data, _) => renderGeneric(data),
    .ping => heartbeat(),
}
```

Nested enums:

```ll
switch event {
    .key(.escape) => dismiss(),
    .key(code) => handle(code),
    .mouse(x, y) => move(x, y),
}
```

## Tuple and named tuple patterns

```ll
let (x, y) = point
let (x: a, y: b) = point

switch pair {
    (0, 0) => origin(),
    (x, 0) => onAxisX(x),
    (0, y) => onAxisY(y),
    (x, y) => plot(x, y),
}
```

## Optional patterns

```ll
switch maybe {
    some(value) => use(value),
    none => default(),
}

if case some(u) = optionalUser {
    greet(u)
}
```

## Type patterns with is

`if` and `switch` support type tests with binding:

```ll
if shape is Circle c {
    print(c.radius)
}

let desc = switch value {
    is String s => s,
    is Int n => "\(n)",
    _ => "other",
}
```

## Literal and range patterns

```ll
switch n {
    0 => "zero",
    1...9 => "digit",
    10... => "many",
}

switch char {
    'a'...'z' => "lower",
    'A'...'Z' => "upper",
    _ => "other",
}
```

## Binding precedence

Names in patterns shadow outer scopes. Use parentheses to nest tuple patterns:

```ll
case (a, (b, c)) => ...
```

## Patterns in for loops

```ll
for (key, value) in map { ... }

for .some(item) in optionalSequence { ... }  // when element is Optional
```

## Destructuring assignment

```ll
var (x, y) = (1, 2)
(x, y) = (y, x)

let Point(x: px, y: py) = point   // when struct exposes memberwise pattern
```

Struct patterns may be enabled when the type is marked `@patternable` or all stored properties are public.

## Combined patterns

```ll
switch result {
    .ok(value) where value.count > 0 => process(value),
    .ok(_) => skip(),
    .err(e) => handle(e),
}
```

## exhaustiveness checking

The compiler verifies:

- All enum cases covered, or `_` present with justification lint.
- Optional `some`/`none` pairs.
- Boolean switches use `true`/`false` or `_`.

Non-exhaustive switches are compile errors for enums; warnings for open integers with `_` missing.

## vs C# switch expressions

LLLang uses `=>` arms exclusively in expression switches; there is no `case x:` fall-through syntax.

## Summary

| Rule | Detail |
|------|--------|
| Binding | Implicit in patterns |
| Enum syntax | `.variant(x)` |
| Guards | `where` after pattern |
| Exhaustive | Required |
| break | Not used in switch |

Next: [Attributes](14-attributes.md).
