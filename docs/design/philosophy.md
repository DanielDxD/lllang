# Design philosophy

LLLang is a compiled, statically typed, object-oriented language for teams that want Swift-like clarity and C#-like tooling without carrying every historical compromise from either ecosystem. The language optimizes for **predictable performance**, **honest error handling**, and **machine-readable project structure** so humans and LLMs can share the same source of truth.

## Pillars

### Performance without a garbage collector

Reference types use **automatic reference counting (ARC)**. Value types (`struct`, `enum`) use copy semantics with copy-on-write collections where appropriate. There is no tracing GC pause; lifetimes are deterministic and `deinit` runs at last release.

The default calling model favors **static dispatch** and monomorphized generics. Dynamic dispatch is explicit (protocol witness tables, `open` members) rather than the default for every virtual call.

Dependencies ship as **prebuilt native libraries** (`.bll`) with separate type stubs (`.sbl`), so application builds link binaries instead of recompiling transitive source trees.

### Developer experience that respects attention

Syntax removes ceremony that does not buy safety:

- No required semicolons; newlines terminate statements when unambiguous.
- **Primary constructors** on structs and classes keep data and initialization colocated.
- **Expression-oriented** `switch` and `if` with `=>` arms reduce temporary variables.
- **Typed throws** (`throws(OrderError)`) document failure modes in the signature.
- **No null** — only `Optional` with `nil`, preserving local reasoning.

Local type inference is intentional: types are inferred where obvious; public APIs and non-trivial expressions require annotations or contextual types from protocols.

### LLM-ready by design

Modern development increasingly pairs humans with agents. LLLang treats that as a first-class requirement:

- **`.mll` files** pack module summaries, public API surfaces, and constraints in **TOON** (Token-Oriented Object Notation) for compact, parse-friendly context.
- **`Std.Llm`** (planned runtime) exposes structured tools, schema-bound decoding, and agent workflows aligned with the type system.
- Documentation and specs are **English**, stable, and cross-linked so agents can anchor on `grammar.ebnf` and ADRs instead of guessing runtime behavior.

### Predictability over magic

Features that hide costs or break local reasoning are opt-in or absent:

- **`unsafe`** gates pointer arithmetic, unchecked memory, and foreign calls without pretending they are safe.
- **Conditional compilation** (`#if`) is explicit; no silent platform forks in the type checker.
- **Error handling** is typed and visible at call sites (`try`, `catch`), not a side channel of unchecked exceptions.

## Non-goals

These are intentional exclusions, not backlog items:

| Non-goal | Rationale |
|----------|-----------|
| Garbage collection | Latency predictability and deterministic teardown |
| Dynamic typing | Safety and optimization require static types |
| Nullable reference types as a bolt-on | Optionals from the ground up |
| Shipping dependency source into consumer repos | Reproducible builds via `.bll` + lockfiles |
| Swift-style external argument labels at definitions | Simpler parsers, clearer generated APIs |
| Recompiling the world on every `lang build` | Developer time and CI cost |

## How decisions are recorded

Architecture choices live in [ADRs](adr/). When syntax and philosophy disagree, **the spec wins** until an ADR explicitly changes direction. Phase 1 prioritizes specification and syntax tooling; runtime behavior documented in the language guide may be marked *planned* until Phase 2 lands codegen.

## Audience

LLLang targets:

- Teams building **CLI tools, services, and agents** that need native speed and strong types.
- Authors of **libraries** who want binary distribution with stable ABIs per target.
- **Educators and spec readers** who prefer a smaller, modernized Swift/C# blend over learning two full platforms.

Success looks like: a developer reads the README sample, finds the same constructs in `grammar.ebnf`, runs `lang check` and the LSP, and gets consistent diagnostics — before native binaries exist.

## Related reading

- [Swift vs C# vs LLLang](swift-vs-csharp-vs-lllang.md)
- [Toolchain architecture](toolchain-architecture.md)
- [Formal grammar](../spec/grammar.ebnf)
- [Roadmap](../../ROADMAP.md)
