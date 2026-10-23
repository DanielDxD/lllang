# ADR 009 — Access control (Swift-like, no `protected`)

## Status

Accepted (Phase 1.1)

## Context

Language guides and the design comparison table disagreed:

- [swift-vs-csharp-vs-lllang.md](../swift-vs-csharp-vs-lllang.md) and [grammar.ebnf](../../spec/grammar.ebnf) describe a **Swift-like** set: `public` / `internal` / `fileprivate` / `private`.
- [03-declarations.md](../../language/03-declarations.md) / [06-structs-classes-enums.md](../../language/06-structs-classes-enums.md) documented C#-style `protected` and denied `fileprivate` / `open`.

Agents and users need one story.

## Decision

1. **Modifiers:** `open` | `public` | `internal` (default) | `fileprivate` | `private`.
2. **No `protected`.** Using `protected` is a compile error (or hard diagnostic) that suggests `fileprivate` (same-file) or `public` (cross-file subclass API).
3. **`open`** marks types/members that may be subclassed/overridden from other packages (vtable). Non-`open` `public` class members are not overridable across package boundaries (see ADR 002).
4. **`private(set)`** (and `fileprivate(set)` / `internal(set)`) narrows setter visibility relative to the getter. Getter uses the leading access modifier; setter uses the parenthesized one.
5. Published `.sbl` exports only `public` / `open` API (`internal` and `fileprivate` never ship).

## Consequences

- Update language guides and keywords table to match this ADR.
- Parser/sema enforce `private(set)` and reject `protected`.
- Custom `subscript` / `operator` declarations stay Later unless implemented separately in Phase 1.1a.
