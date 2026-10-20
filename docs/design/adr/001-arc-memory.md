# ADR 001: ARC memory management

**Status:** Accepted  
**Date:** 2026-03-20  
**Deciders:** LLLang core team

## Context

LLLang targets native performance for CLI tools, services, and libraries. A tracing garbage collector simplifies some language designs but introduces pause times, write barriers, and non-deterministic finalization — poor fits for latency-sensitive agents and systems glue code.

Swift demonstrates that **automatic reference counting (ARC)** plus value types can deliver predictable performance without a GC. C# relies on GC but adds struct value semantics as an partial escape hatch. LLLang must choose a primary model for heap-allocated `class` instances and shared ownership.

Requirements:

- Deterministic teardown for resources (files, sockets, GPU handles).
- No stop-the-world collections in the default runtime.
- Clear story for cycles (`weak`, `unowned`).
- FFI interop without relocating objects unexpectedly.

## Decision

Adopt **ARC as the sole automatic memory management strategy** for reference types (`class`, actors when introduced). Value types (`struct`, `enum`) use move/copy semantics with copy-on-write (COW) for large containers in the standard library.

Rules:

1. Every `class` instance carries a reference count manipulated at retain points (assignment, capture, call, return).
2. **`deinit` runs synchronously** when the strong count reaches zero on the owning thread unless documented otherwise for global teardown.
3. **Reference cycles** are broken only via `weak` or `unowned`; the compiler emits `LL3002` when static analysis detects likely cycles (planned Phase 3).
4. **Structs never use ARC** for their inline storage; nested reference types inside structs retain their targets independently.
5. **Unsafe code** may opt out of retain/release for documented spans; safe code cannot construct unowned dangling references without `unsafe`.

There will be **no tracing GC** and no hybrid GC/ARC mode.

## Consequences

### Positive

- Predictable latency and memory use for server and agent workloads.
- Finalizers (`deinit`) are usable for resource management when paired with `weak` breaks.
- LLVM lowering maps cleanly to retain/release intrinsics or outlined runtime calls.
- Semantic model aligns with Swift developers; docs and skills transfer concepts easily.

### Negative

- Cycle prevention is the developer’s responsibility (with compiler help later).
- Atomic refcounting on concurrent retains adds overhead vs single-threaded ownership (mitigated by move-only patterns and `@MainActor`-style isolation later).
- FFI requires explicit ownership conventions at boundaries (`owned`, `borrowed`, `unsafe`).

### Follow-ups

- Implement retain/release insertion in MIR (Phase 2).
- Document standard library COW types (`Array`, `String`, `Dictionary`).
- Add diagnostics `LL3000`–`LL3004` in the borrow/ARC pass.

## References

- [Memory model (guide)](../../language/11-memory-model.md)
- [Philosophy](../philosophy.md)
- [Swift vs C# vs LLLang](../swift-vs-csharp-vs-lllang.md)
