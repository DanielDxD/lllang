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
- Published libs are **portable `.bll` (LLVM bitcode, BLL v2)** + `.sbl` — not host-native objects. Consumers finalize for `--target` (official: darwin/arm64, linux/amd64|arm64, windows/amd64|arm64). See ADR 010.
- Docs and public samples are **English**.

## Compiler crates

- `crates/lllang-syntax` — lexer/parser
- `crates/lllang-manifest` — `Package.ll` / `Solution.ll` evaluation
- `crates/lllang-sema` — name resolution, type check, HIR (LL1xxx / LL2xxx)
- `crates/lllang-mir` — HIR → MIR + ARC inserts; `--emit-mir`
- `crates/lllang-llvm` — inkwell LLVM 22 codegen + link (`LLVM_SYS_221_PREFIX`)
- `runtime/lllang_rt.c` — ARC, strings, arrays, print, traps, concurrency, sqlite
- `crates/lllang-lsp` — syntax + sema diagnostics
- `crates/lang` — `check`, `build`, `run`, `lsp`, `test`, `fmt`, `pack`, …

Requires **LLVM 22** (`brew install llvm@22`, export `LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22`).

## Phase status (see [ROADMAP.md](ROADMAP.md))

- Phases 1–4: vertical slice (parse → LLVM, LSP, templates, registry MVP)
- Phase 5–6: core execution + stdlib builtins
- Phase 7–9: templates, inheritance/FFI/testing polish, toolchain trust
- Phase 10: concurrency MVP, SQL sqlite, weak/unowned, existentials, Codable containers
- Phase 1.1: language polish done (extensions, casts/`is`, Env/Identity, concurrency honesty, docs)
- **Phases 1.2–1.6 MVP done** (+ **1.6.1** portable BLL v2 / Go-style `--target`). HOFs/closures/TaskGroup/`@interop` C, HMAC/Arena, `HttpServer`, registry host, REPL/DAP, WASM policy. Open: TLS, SQL dialect #2, multi-lang interop beyond C. Prefer ROADMAP; do not invent `tmp/1.2` (Phase 1.7) features yet.

When unsure, prefer updating the **specification** / ROADMAP over inventing runtime behavior. Docs must label **Shipped MVP** vs unshipped — never describe paper APIs as callable.
