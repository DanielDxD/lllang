# Lexical structure

LLLang source files use the `.ll` extension. The lexer turns a byte stream into tokens while respecting Unicode identifiers, nested comments, and string interpolation. This chapter describes how text is broken into meaningful pieces before parsing.

**Related:** [Types](02-types.md) · [Declarations](03-declarations.md) · [Keywords](../spec/keywords.md)

## Source files and encoding

- Source is **UTF-8**. A leading BOM is ignored.
- Line endings may be `\n`, `\r\n`, or `\r`; the compiler normalizes to logical newlines.
- Files belong to a **package** and optional **namespace** (see [Namespaces & modules](15-namespaces-modules.md)).

## Whitespace and statement termination

Whitespace (spaces, tabs, form feeds) separates tokens but is not otherwise significant.

**Semicolons are never required.** A newline terminates a statement when the parser can unambiguously end the current construct. You may still write `;` to put multiple short statements on one line:

```ll
let a = 1; let b = 2
```

Continuation lines are formed when an expression clearly continues across a newline (e.g. after `(`, `[`, `{`, `,`, `=>`, or a binary operator at end of line).

## Comments

```ll
// Line comment to end of line

/* Block comment */
/* Nested /* comments */ are supported */

/// Documentation comment (Markdown body)
/// - Bullet lists and **emphasis** render in IDE tooltips
```

Documentation comments attach to the **next** declaration. Use them on public APIs exported from libraries.

## Identifiers

Identifiers start with a letter or `_`, then letters, digits, or `_`. Unicode letters are allowed (`café`, `用户`). By convention:

| Style | Use |
|-------|-----|
| `PascalCase` | Types, protocols, enums, namespaces |
| `camelCase` | Functions, variables, parameters |
| `SCREAMING_SNAKE` | Constants in `const` blocks (optional) |

Keywords are reserved; see [keywords](../spec/keywords.md). You cannot define types whose names collide with keywords unless escaped with backticks (`` `class` ``) — discouraged except for interop shims.

## Literals

### Integers

Decimal, binary (`0b1010`), octal (`0o755`), and hex (`0xFF`). Underscores improve readability:

```ll
let million = 1_000_000
let mask = 0xFF_FF_FF_00
let flags = 0b0001_0010
```

Default integer type is inferred locally as `Int` unless a suffix or context demands otherwise. Suffixes: `u8`, `i32`, `u64`, etc.

### Floating point

```ll
let pi = 3.14159
let avogadro = 6.022_140_76e23
let tiny = 1e-9f32
```

`Decimal` literals use `d` suffix or context: `19.99d`.

### Booleans and nil

```ll
let ok = true
let fail = false
let empty: String? = nil   // only inside Optional types
```

There is **no null** — only `nil` for optionals.

### Characters and strings

Single-quoted **character** literals are Unicode scalar values:

```ll
let newline = '\n'
let heart = '❤'
```

Double-quoted **strings** are UTF-8 `String` values with escape sequences `\n`, `\t`, `\"`, `\\`, `\u{1F600}`.

**Raw strings** use `#` delimiters; `#` count must match on both sides:

```ll
let path = #"C:\Users\dev\file.ll"#
let json = ##"{ "key": "value" }"##
```

### Interpolation

```ll
let name = "Ada"
let msg = "Hello, \(name)!"
let price = 19.5
let receipt = "Total: \(price, .fixed(2)) \(currency, .uppercased)"
```

Format options follow the interpolated expression after a comma (see [Expressions](04-expressions.md)).

## Operators and punctuation

LLLang uses familiar C-family operators with some expression-oriented additions (`=>`, `??`, `?.`). Precedence is documented in [operators](../spec/operators.md). Notable tokens:

| Token | Role |
|-------|------|
| `=>` | Switch/if arms, lambda bodies, computed property shorthand |
| `??` | Nil-coalescing |
| `?.` `?[]` | Optional chaining |
| `..<` `...` | Range operators |
| `@` | Attributes |
| `#` | Conditional compilation (`#if DEBUG`) |

## Keywords (overview)

LLLang blends Swift-like and C#-like forms without copying either wholesale. There is **no** `new`, `null`, `mutating`, `associatedtype`, `any`, `some`, `case` (for enums), or Swift-style external argument labels at definition sites.

Construction uses `Type(args)`. Protocols are generic with angle brackets: `protocol Container<Element>`.

## Conditional compilation

```ll
#if DEBUG
    Log.trace("building debug")
#elseif PROFILE
    Profiler.enable()
#else
    // release
#endif
```

Symbols defined in manifest `[build]` features or `-D NAME` on the CLI are visible to `#if`.

## Tokenization of string views

The standard library exposes **views** over strings — `chars`, `bytes`, `graphemes` — without a `String.Index` type. Lexically, string literals are still single tokens; view APIs are runtime (see [Types](02-types.md)).

## Layout-sensitive parsing

The parser is largely **brace-based**, not indentation-based. However, multi-line `switch` and `if` expression arms must use consistent `=>` alignment for readability; the formatter enforces style.

## End of file

A file may end with or without a trailing newline. The parser accepts an optional semicolon before `}` in macro-generated code for compatibility with code generators.

## Summary

| Rule | Behavior |
|------|----------|
| Statement ends | Newline (primary) or `;` |
| Null | Not in the language |
| Numeric separators | `_` allowed |
| Trailing commas | Allowed in lists, tuples, calls |
| Doc comments | `///` Markdown |

Next: learn how those tokens form the [type system](02-types.md).
