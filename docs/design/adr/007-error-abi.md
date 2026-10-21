# ADR 007 — Throwing call ABI (Phase 2)

## Status

Accepted (Phase 2 vertical slice).

## Context

LLLang uses typed `throws(E)`. The runtime needs a calling convention that does not depend on DWARF exceptions yet.

## Decision

- Callees with `throws(E)` take an extra trailing parameter `__err_out: *mut *mut void` (pointer to an error-object slot).
- On success the callee stores `null` into `*__err_out` and returns the normal value.
- On `throw` the callee boxes the error payload (`ll_box_bytes` / retain) into `*__err_out` and returns a zero/poison value for the result type.
- Callers allocate an err slot, pass its address, then `LoadErrFlag` / `LoadErrValue` to implement `try?`, `catch`, etc.
- Entry `ll_main` omits `__err_out` (C runtime calls it with no args); uncaught throws trap.

## Consequences

- Simple, portable, works with the C runtime linker.
- Stack unwinding / DWARF exceptions remain future work (Phase 4+).
