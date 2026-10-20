# ADR 002: Static dispatch by default

**Status:** Accepted  
**Date:** 2026-03-20  
**Deciders:** LLLang core team

## Context

Dynamic dispatch (virtual methods, interface calls through fat pointers) enables polymorphism but prevents inlining, devirtualization, and specialized generics at compile time. C# defaults to virtual interface dispatch; Swift mixes witness tables and class vtables.

LLLang prioritizes **performance predictability** and LLVM-friendly IR. Libraries distributed as `.bll` binaries also need stable ABIs for exported functions while keeping internal calls optimizable.

## Decision

**Static dispatch is the default** for functions, methods, and generic protocol requirements when the concrete type is known at compile time.

Dynamic behavior requires explicit markers:

| Mechanism | Effect |
|-----------|--------|
| `open class` / `open func` | Allows override in subclasses; vtable dispatch for overrides |
| Protocol value (existential) — planned | Witness table dispatch through constrained generic or boxed protocol |
| `@dynamicCallable` / attributes (if added) | Opt-in reflection-style paths |

Generic functions and methods are **monomorphized** per specialization unless explicitly exported with a stable generic ABI in `.sbl` (advanced interop).

Protocol extensions provide default implementations resolved at compile time when the concrete type is known (static witness), not runtime message send.

## Consequences

### Positive

- LLVM can inline and specialize aggressively in hot paths.
- Smaller binaries when unused virtual slots are not emitted.
- Mental model: pay for dynamic dispatch only where marked.

### Negative

- Developers coming from C# may expect implicit `virtual` on interfaces; LLLang requires `open` or protocol existential patterns.
- Protocol existentials need careful design to avoid Swift-style complexity (`any`/`some`); v1 focuses on generics + protocols with type parameters.

### Follow-ups

- Specify ABI for `open` vs `final` in `.sbl` export metadata.
- Codegen tests ensuring direct calls for non-open methods.
- Document pattern: `func render<T: Drawable>(x: T)` over heterogeneous collections in Phase 2.

## References

- [Toolchain architecture](../toolchain-architecture.md)
- [Protocols & extensions (guide)](../../language/07-protocols-extensions.md)
