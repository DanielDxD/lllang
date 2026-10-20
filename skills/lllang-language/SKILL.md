---
name: lllang-language
description: Write and review LLLang (.ll) source using improved syntax — primary constructors, expression switch, typed throws, no semicolons/null/new. Use when editing language examples, specs, or application code in this repo.
---

# LLLang language

## Non-negotiable syntax

- No statement semicolons; newline-terminated blocks.
- No `null`, no `new`, no Swift external argument labels (no `_` label placeholders on parameters).
- Construct values with `Type(args)` or literals.
- Enums list variants without the `case` keyword: `ok(T), err(E), idle`.
- `switch` is an expression: arms use `pattern => expr` with trailing commas; add `_` only for open ranges.
- Lambdas: `(x) => expr` or `(a, b) => { ... }`.
- Computed members: `var total: Decimal => items.sum((i) => i.amount)`.
- Typed errors: `throws(OrderError)` on functions; `throw .variant` when context types the enum.
- Propagation: `try`, `try?`, `try!` (tests/invariants only).
- Recovery: `expr catch (e) => fallback` or braced `catch` body.
- Async: `async func`, `await`, `try await` (await before try on same call).
- Classes: `virtual` / `override` / `sealed`; default dispatch is static unless marked virtual.
- Protocols: `protocol Name<T> { }` with angle generic parameters (no `associatedtype`, no `any`/`some`).
- Patterns bind by default: `.circle(r)` not `.circle(let r)`.

## Declarations

- Primary constructor parameters on struct/class headers become stored properties.
- Protocol conformances after the type name: `struct User(...): Codable, Equatable`.
- `import Std.Module` for stdlib; `Std.Core` is implicit.
- `namespace` segments use dots; align with package id when practical.

## When unsure

1. [docs/spec/grammar.ebnf](../../docs/spec/grammar.ebnf)
2. [docs/design/swift-vs-csharp-vs-lllang.md](../../docs/design/swift-vs-csharp-vs-lllang.md)
3. Language guide under `docs/language/`

Prefer updating the spec over inventing runtime behavior. Keep public samples valid for `lang check`.
