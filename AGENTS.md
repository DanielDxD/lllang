# Agent guide for LLLang

This repository defines **LLLang**, a compiled OOP language with Swift/C#-inspired (but improved) syntax, ARC memory, and LLM-oriented tooling.

## Read first

1. [README.md](README.md) — product overview and sample
2. [docs/design/philosophy.md](docs/design/philosophy.md)
3. [docs/design/swift-vs-csharp-vs-lllang.md](docs/design/swift-vs-csharp-vs-lllang.md) — intentional differences
4. [docs/spec/grammar.ebnf](docs/spec/grammar.ebnf)
5. Skills under [`skills/`](skills/) (also `.cursor/skills/`)

## Skills

| Skill | Use when |
|-------|----------|
| `lllang-language` | Writing or reviewing `.ll` source |
| `lllang-cli` | `lang` commands, templates, manifests |
| `lllang-packages` | Dependencies, `.bll`/`.sbl`, store layout |
| `lllang-json-toon` | JSON/TOON encode-decode |
| `lllang-llm` | `.mll`, `Std.Llm`, agents |
| `lllang-stdlib` | Standard library APIs |
| `lllang-editor-tooling` | LSP, TextMate, highlighting |

## Conventions agents must follow

- **No semicolons** required; no `null`; no `new`; no Swift external argument labels / `_`.
- Prefer **primary constructors**, expression `switch`/`if`, typed `throws(E)`, and `catch` expression handlers.
- Package manifests are **`Package.ll`** (SwiftPM-style DSL), not TOML. Monorepos use root **`Solution.ll`** (.NET solution-style) plus path deps between packages. Lockfile: **`Package.lock`**.
- Keep examples under `examples/` valid against the grammar.
- Do not invent GC, `node_modules`-style dependency trees, or recompile-from-source for published libraries.
- Docs and public samples are **English**.

## Phase-2 code

- `crates/lllang-syntax` — lexer/parser
- `crates/lllang-manifest` — `Package.ll` / `Solution.ll` evaluation
- `crates/lllang-sema` — name resolution, type check, HIR (LL1xxx / LL2xxx)
- `crates/lllang-mir` — HIR → MIR + ARC inserts; `--emit-mir`
- `crates/lllang-llvm` — inkwell LLVM 22 codegen + link (`LLVM_SYS_221_PREFIX`)
- `runtime/lllang_rt.c` — ARC, strings, arrays, print, traps
- `crates/lllang-lsp` — syntax + sema diagnostics
- `crates/lang` — `check`, `build`, `run`, `lsp`, `--version`

Requires **LLVM 22** (`brew install llvm@22`, export `LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22`).

Phase 2.1 done: `lang fmt`, LSP go-to-def / rename / inlays, ABI/COW/vtable polish.
Phase 3 MVP: `lang pack` / store / `Package.lock`, Std.Json/Toon/Http/Async/Llm for examples.
Phase 4 MVP: typed LSP completion + LL2005 code actions, file registry publish/search, `lang new` / `bench`, tree-sitter stub.

When unsure, prefer updating the **specification** over inventing runtime behavior.
