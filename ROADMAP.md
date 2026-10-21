# Roadmap

## Phase 1 — Spec, docs, syntax, LSP shell

- [x] Language specification and design docs (English)
- [x] File formats: `.ll`, `.bll`, `.sbl`, `.mll`
- [x] CLI & package manager design (`lang`)
- [x] Standard library API surface (documented)
- [x] Examples and agent skills
- [x] `lllang-syntax` lexer/parser + diagnostics
- [x] Minimal LSP (`lang lsp`) + VS Code / web highlighting

## Phase 2 — Semantic analysis & LLVM (vertical slice)

- [x] Name resolution and type checker (`lllang-sema`, diagnostics LL1xxx / LL2xxx)
- [x] HIR → MIR with ARC retain/release insertion (`lllang-mir`)
- [x] LLVM 22 codegen via `inkwell` (`lllang-llvm`) + C runtime (`runtime/lllang_rt.c`)
- [x] `lang check` with sema; `lang build` / `lang run` for native binaries
- [x] Examples: `hello`, `errors`, `shapes-oop`, `shop-solution` (path deps / `Solution.ll`)

### Phase 2.1 — tooling pass

- [x] Formatter (`lang fmt`)
- [x] Fuller LSP (go-to-def, rename, inlay hints) on top of sema
- [x] Richer throwing ABI / COW collections / class vtables polish

## Phase 3 — Stdlib runtime & packages

- [x] Implement `Std.*` modules (Json, Toon, Async, Llm, …) — MVP for examples
- [x] Global package store + resolver + `Package.lock`
- [x] `.bll` packing (`lang pack`) and registry protocol (local store / search)
- [x] Cross-target builds (host + `--target` / `lang sdk` MVP)

## Phase 4 — DX polish & ecosystem

- [x] Full LSP (type-aware completion, code actions) — MVP: members after `.`, LL2005 `try` quick-fix
- [x] Tree-sitter grammar stub (`editors/tree-sitter-lllang`; JetBrains still optional later)
- [x] Registry client + file-backed mock (`lang publish` / `search`, `LANG_REGISTRY`; no public host yet)
- [x] MCP / agent templates (`lang new`) and `examples/mcp-echo`
- [x] Benchmarks MVP (`lang bench` + `docs/toolchain/benchmarks.md`)

## Non-goals

- Garbage collection
- Dynamic typing / optional types that erase null safety
- Shipping dependency *source* into projects (libraries remain prebuilt `.bll`)
