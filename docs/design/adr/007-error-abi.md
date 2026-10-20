# ADR 007 — Throwing call ABI (Phase 2)

## Status

Accepted (Phase 2 vertical slice).

## Context

LLLang uses typed `throws(E)`. The runtime needs a calling convention that does not depend on DWARF exceptions yet.

## Decision

- Callees with `throws(E)` take an extra trailing parameter `__err_out: *mut *mut void` (pointer to an error-object slot).
- On success the callee stores `null` into `*__err_out` and returns the normal value.
- On `throw` the callee stores a non-null sentinel (Phase 2) and returns a zero/poison value for the result type.
- Callers allocate an err slot, pass its address, then `LoadErrFlag` (non-null check) to implement `try?`, `catch`, etc.

## Consequences

- Simple, portable, works with the C runtime linker.
- Full boxed error payloads and stack unwinding remain future work (see ROADMAP Phase 2.1).
