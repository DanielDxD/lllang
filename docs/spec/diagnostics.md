# Diagnostic codes

LLLang diagnostics use stable codes for tooling, CI filters, and agent skills. Format:

```text
LL<Sphase><nnnn>: <message>
```

| Phase digit | Range | Subsystem |
|-------------|-------|-----------|
| `0` | `LL0001`–`LL0999` | Lexer and parser (Phase 1) |
| `1` | `LL1000`–`LL1999` | Name resolution (planned) |
| `2` | `LL2000`–`LL2999` | Type checking (planned) |
| `3` | `LL3000`–`LL3999` | Memory / ARC / exclusivity (planned) |
| `4` | `LL4000`–`LL4999` | Codegen and linker (planned) |
| `5` | `LL5000`– `LL5999` | Packages and manifests (planned) |
| `6` | `LL6000`–`LL6999` | LSP and editor adapters (planned) |

Severity: **Error** stops compilation; **Warning** allows continue; **Note** is informational.

---

## Phase 0 — Syntax (`LL0xxx`)

### Lexer (`LL0001`–`LL0099`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0001 | Error | Invalid UTF-8 byte sequence at offset {off} |
| LL0002 | Error | Unterminated block comment |
| LL0003 | Error | Unterminated string literal |
| LL0004 | Error | Unterminated raw string delimiter |
| LL0005 | Error | Invalid escape sequence `\{seq}` |
| LL0006 | Error | Invalid Unicode scalar in character literal |
| LL0007 | Error | Empty character literal |
| LL0008 | Error | Invalid numeric literal `{lexeme}` |
| LL0009 | Error | Integer literal overflow for suffix `{suffix}` |
| LL0010 | Error | Unterminated regex literal |
| LL0011 | Warning | Ignored shebang line |
| LL0012 | Error | Invalid identifier after backtick |
| LL0013 | Error | Nested interpolation depth exceeds limit |
| LL0014 | Error | Unknown string interpolation format `{name}` |

### Parser — general (`LL0100`–`LL0199`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0100 | Error | Unexpected token `{tok}`; expected {expected} |
| LL0101 | Error | Unexpected end of file |
| LL0102 | Error | Extraneous tokens after top-level declaration |
| LL0103 | Error | Invalid declaration context for `{construct}` |
| LL0104 | Error | Duplicate modifier `{mod}` |
| LL0105 | Error | Access modifier `{mod}` is not valid here |
| LL0106 | Error | `private(set)` must follow access modifier on member |
| LL0107 | Error | Trailing closure not allowed in this position |

### Parser — types & generics (`LL0200`–`LL0299`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0200 | Error | Expected type after `:` |
| LL0201 | Error | Expected `>` to close generic argument list |
| LL0202 | Error | Expected `>` to close generic parameter list |
| LL0203 | Error | Generic parameter list required before `where` |
| LL0204 | Error | Empty tuple type `()` is not allowed |
| LL0205 | Error | Protocol inheritance list must be comma-separated |
| LL0206 | Error | `throws` type list must name at least one type when `(` is present |

### Parser — declarations (`LL0300`–`LL0399`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0300 | Error | Expected `{` to start type body |
| LL0301 | Error | Primary constructor parameters must declare types |
| LL0302 | Error | `deinit` is only valid on classes |
| LL0303 | Error | Enum case `{name}` cannot have both payload and raw value |
| LL0304 | Error | `case` keyword is not used for enum cases; write `{name}` directly |
| LL0305 | Error | Extension cannot add stored properties |
| LL0306 | Error | Duplicate member `{name}` in type body |
| LL0307 | Error | `init` cannot specify return type |

### Parser — patterns & control flow (`LL0400`–`LL0499`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0400 | Error | Expected pattern |
| LL0401 | Error | Expected `=>` in switch arm |
| LL0402 | Error | `default` arm must use `=>` in expression switch |
| LL0403 | Error | `guard` must have `else` branch |
| LL0404 | Error | Unlabeled `break` outside loop or switch |
| LL0405 | Error | `catch` clause must follow `do` block |
| LL0406 | Error | Expression `if` requires `=>` on each branch |

### Parser — expressions (`LL0500`–`LL0599`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0500 | Error | Expected expression |
| LL0501 | Error | Assignment operator requires assignable expression |
| LL0502 | Error | Closure parameter list malformed |
| LL0503 | Error | `unsafe` block must contain `{ ... }` |
| LL0504 | Error | `try` expression missing following expression |
| LL0505 | Error | Invalid optional chain suffix |

### Conditional compilation (`LL0600`–`LL0699`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0600 | Error | `#if` without matching `#endif` |
| LL0601 | Error | `#elseif` or `#else` without preceding `#if` |
| LL0602 | Error | Unknown compile-time symbol `{sym}` |
| LL0603 | Warning | Empty `#if` branch |

### Attributes & imports (`LL0700`–`LL0799`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL0700 | Error | Unknown attribute `{name}` at parse time |
| LL0701 | Error | Malformed attribute argument list |
| LL0702 | Error | Import path must be a qualified name |
| LL0703 | Error | `import ... as` requires identifier alias |

---

## Planned — Name resolution (`LL1xxx`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL1000 | Error | Cannot find type `{name}` in scope |
| LL1001 | Error | Cannot find value `{name}` in scope |
| LL1002 | Error | Ambiguous reference to `{name}` |
| LL1003 | Error | Import `{module}` not found |
| LL1004 | Error | Symbol `{name}` is private to `{module}` |
| LL1005 | Error | Circular inheritance involving `{type}` |
| LL1006 | Error | Extension cannot override existing member `{name}` |

## Planned — Type checking (`LL2xxx`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL2000 | Error | Type mismatch: expected `{expected}`, found `{found}` |
| LL2001 | Error | Cannot infer type for `{desc}`; add annotation |
| LL2002 | Error | Value of optional type must be unwrapped |
| LL2003 | Error | `nil` is only valid for optional `{type}?` |
| LL2004 | Error | Function throws `{E}` but body throws `{F}` |
| LL2005 | Error | Call requires `try`; function throws `{E}` |
| LL2006 | Error | Generic type `{G}` could not be specialized |
| LL2007 | Error | Type `{T}` does not conform to protocol `{P}` |
| LL2008 | Error | Switch must be exhaustive |
| LL2009 | Warning | Unreachable switch arm |
| LL2010 | Error | `weak` may only apply to class-bound optional |
| LL2011 | Error | Escaping closure captures `inout` parameter |

## Planned — Memory & ARC (`LL3xxx`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL3000 | Error | Use of value `{name}` after move |
| LL3001 | Error | Simultaneous mutable access to `{name}` |
| LL3002 | Error | Reference cycle requires `weak` or `unowned` |
| LL3003 | Error | `deinit` accessed freed memory |
| LL3004 | Warning | Large value copy; consider `inout` or reference type |

## Planned — Codegen & link (`LL4xxx`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL4000 | Error | Undefined symbol `{sym}` at link time |
| LL4001 | Error | ABI mismatch for library `{lib}` |
| LL4002 | Error | Unsupported target `{triple}` |

## Planned — Packages (`LL5xxx`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL5000 | Error | Manifest parse error: {detail} |
| LL5001 | Error | No compatible `.bll` for target `{triple}` |
| LL5002 | Error | Version conflict on `{pkg}` |
| LL5003 | Error | Checksum mismatch for `{pkg}` |

## Planned — LSP (`LL6xxx`)

| Code | Severity | Message (template) |
|------|----------|-------------------|
| LL6000 | Warning | Document out of date; reparsing |
| LL6001 | Error | Internal LSP error: {detail} |

---

## Emitting diagnostics

Phase 1 (`lang check`, `lllang-lsp`) emits **LL0xxx** only. Each diagnostic includes:

- `code` — stable identifier (e.g. `LL0304`)
- `message` — human-readable text
- `span` — start/end byte offsets in UTF-8 source
- `severity` — `error` | `warning` | `note`
- `related` — optional secondary spans (later phases)

Tools should not parse message text; use `code` for filtering and documentation links (`docs/spec/diagnostics.md#LL0304`).

## Adding codes

1. Reserve the next free number in the subsystem block.
2. Document the template here before implementing in `lllang-syntax`.
3. Add a snapshot test in the syntax crate for the diagnostic shape.
