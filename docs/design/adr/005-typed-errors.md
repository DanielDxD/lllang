# ADR 005: Typed errors (`throws(E)`)

**Status:** Accepted  
**Date:** 2026-03-20  
**Deciders:** LLLang core team

## Context

Untyped throws (Swift’s `throws` to `any Error`) and unchecked exceptions (C#) make control flow invisible at call sites. Callers cannot know which failures to handle without reading implementations or documentation.

LLLang enums already model sum types without a `case` keyword; error enums conform to a standard **`Error`** protocol. The language should surface failure types in function signatures and expression-level handling.

## Decision

1. Functions that may fail declare **`throws(ErrorType)`** or **`throws(E1, E2)`** when multiple error types propagate. Omitted `throws` means the function does not propagate typed errors to callers (it may still trap on programmer error).
2. **`throw`** uses enum case syntax: `throw .outOfStock(sku: id)` inside a function throwing compatible types.
3. Call sites use **`try`** for required handling or propagation rules defined in the type checker (Phase 2).
4. Expression handlers: **`catch (pat) => expr`** suffix and `do { } catch { }` blocks bind typed patterns.
5. Error enums are **exhaustive** in `switch`; associated payloads bind with pattern syntax shared by expression `switch`.

There is **no** parallel unchecked exception hierarchy and no `null` error channel.

## Consequences

### Positive

- API contracts document failure modes; agents index them in `.mll`.
- Compiler can verify handling (`LL2004`, `LL2005`) and exhaustiveness (`LL2008`).
- Maps cleanly to `Result`-like types in stdlib without language-level `Result` keyword requirement.

### Negative

- Ergonomics cost: typed propagation must be widened explicitly or mapped at boundaries.
- Interop with C/HTTP status codes needs adapter enums at FFI layer.

### Follow-ups

- Specify subtyping rules when `throws(A)` calls `throws(B)` (same enum, wrapped errors).
- Standard `mapError` helpers in `Std.Error` module.

## References

- [Error handling (guide)](../../language/10-error-handling.md)
- [Grammar: throwsClause](../spec/grammar.ebnf)
- README sample with `throws(OrderError)`
