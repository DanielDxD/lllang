# Swift vs C# vs LLLang

This table summarizes intentional similarities and differences. LLLang is **not** a compatibility layer for either language; it cherry-picks patterns that improve safety, performance, and agent-friendly tooling.

| Feature | Swift | C# | LLLang | Why |
|---------|-------|-----|--------|-----|
| Memory model | ARC + structs; no GC | Tracing GC + structs | ARC; no GC | Predictable latency, deterministic `deinit` |
| Null | Optionals; no null for values | Nullable reference types (optional) | `T?` and `nil` only; no `null` keyword | Single optional story, no legacy reference null |
| Statement terminators | Often inferred; semicolon optional in practice | Semicolon required (mostly) | Newline-terminated; `;` optional | Less noise; matches modern Swift ergonomics |
| Object construction | `Type()` | `new Type()` | `Type(args)` — no `new` | One construction syntax; reads like a call |
| Enum cases | `case name` | `Name = 0,` | `name, payload(...)` — no `case` | Less keyword clutter; uniform with tuples |
| Primary constructors | Limited (memberwise / explicit init) | Primary ctor (C# 12+) | Struct/class `(params)` in header | Fields and invariants visible at declaration |
| Protocols / interfaces | `protocol` + associated types | `interface` | `protocol P<T>` with angle generics | Explicit generic params instead of associated types |
| Existentials | `any P` / `some P` | Interfaces + variance | Concrete generics + planned witness model | Smaller type system surface in v1 |
| Error handling | `throws` + untyped `Error` | Exceptions | `throws(ErrorType)` typed + `Result`-like flows | Errors are part of the API contract |
| Expression switch | Limited; often statement | `switch` expression (recent) | `switch e { pat => expr }` | Expression-oriented control flow by default |
| Pattern matching | `case let` / `where` | `switch` patterns (growing) | `=>` arms, `where`, no enum `case` | Aligns with expression switch syntax |
| Lambdas | `{ x in ... }` | `x => ...` | `(x) => expr` | Familiar to C# users; unambiguous body start |
| Properties | `var` with `{ get set }` | `{ get; set; }` | `var x: T => expr` shorthand + observers | Computed properties without boilerplate |
| Access control | `open/public/internal/fileprivate/private` | `public/internal/protected/private` | Swift-like set; no `protected` | Module boundaries without inheritance coupling |
| Extensions | `extension` | static class extensions | `extension Type { }` | Add APIs without subclassing |
| Attributes | `@objc` etc. | `[Attribute]` | `@Name(args)` | Single token; easy to parse for LSP |
| Unsafe / pointers | `Unsafe*` types | `unsafe` blocks + pointers | `unsafe { }` keyword gate | Clear audit boundary for FFI |
| Package manager | SPM (source) | NuGet | `lang` + `.bll`/`.sbl` prebuilt | Faster CI, no `node_modules`-scale trees |
| LLM metadata | None standard | None standard | `.mll` + TOON | Compact machine context for agents |
| String indexing | `String.Index` | `ReadOnlySpan<char>` | Unicode views (`chars`, `graphemes`) | Avoid index invalidation footguns |
| Concurrency | `async/await`, actors | `async/await`, tasks | `async`/`await`; actors planned | Align with industry without shipping runtime in Phase 1 |
| Conditional compilation | `#if` | `#if` | `#if` / `#elseif` / `#endif` | Cross-platform builds with explicit branches |
| Default dispatch | Witness + class vtable mix | Virtual by default on interfaces | Static default; dynamic when marked | Performance-first ABI story |
| Deinitializer | `deinit` | Finalizers (non-deterministic) | `deinit` on classes | Deterministic resource release |
| Codable / serialization | `Codable` | `System.Text.Json` attributes | `Codable` protocol + `Std.Json`/`Std.Toon` | One protocol; LLM-friendly TOON alongside JSON |
| Tooling entry | `swift` CLI | `dotnet` CLI | `lang` CLI (.NET-style verbs) | Familiar command vocabulary |
| LSP implementation | SourceKit | Roslyn | Rust + `tower-lsp` (Phase 1 syntax) | Single binary pipeline with compiler frontend |

## Reading guide

- Where LLLang matches Swift, it often **removes** keywords (`case`, `new`) or **narrows** types (no untyped throw).
- Where LLLang matches C#, it often **adds** value semantics (structs/enums) and **removes** GC assumptions.
- When in doubt, prefer [grammar.ebnf](../spec/grammar.ebnf) and [philosophy.md](philosophy.md) over analogy to either language.

## Migration expectations

Automatic migration from Swift or C# is a non-goal. Similar syntax is for **learning curve reduction**, not source compatibility. Interop (Phase 2+) targets C ABI and platform libraries via `unsafe` and packaged `.bll` artifacts, not language-level compatibility modes.
