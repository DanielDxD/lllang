# ADR 008 — Object and value layouts (Phase 2)

## Status

Accepted (Phase 2 vertical slice).

## Context

ARC classes, value structs/enums, and optional types need stable LLVM layouts for codegen and the C runtime.

## Decision

| Kind | Layout |
|------|--------|
| Class instance | Heap object: `LlObjectHeader { refcount, type* }` followed by payload fields; API uses **payload pointer** (`ll_alloc_sized` / `ll_retain` / `ll_release`). |
| Struct | LLVM struct of fields, passed/returned by value. |
| Enum | `{ i32 tag, [8 x i64] payload }` (fixed Phase-2 representation). |
| `T?` | `{ i1 present, T value }`. |
| `String` / `Array` | Refcounted runtime handles (`LlString` / `LlArray`). |

## Consequences

- Class field access uses struct GEP on the payload pointer with the named type layout.
- Enum payload slots are coarse (i64 cells); refine when richer ABI is needed.
