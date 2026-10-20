# Roadmap

## Phase 1 — Spec, docs, syntax, LSP shell (current)

- [x] Language specification and design docs (English)
- [x] File formats: `.ll`, `.bll`, `.sbl`, `.mll`
- [x] CLI & package manager design (`lang`)
- [x] Standard library API surface (documented)
- [x] Examples and agent skills
- [x] `lllang-syntax` lexer/parser + diagnostics
- [x] Minimal LSP (`lang lsp`) + VS Code / web highlighting

## Phase 2 — Semantic analysis & LLVM

- [ ] Name resolution, type checker, borrow/ARC analysis
- [ ] LLVM codegen via `inkwell` (LLVM 22)
- [ ] Runtime: ARC, COW collections, panic/trap paths
- [ ] `lang build` / `lang run` producing native binaries
- [ ] Formatter (`lang fmt`) and fuller LSP (go-to-def, rename, inlay hints)

## Phase 3 — Stdlib runtime & packages

- [ ] Implement `Std.*` modules (Json, Toon, Async, Llm, …)
- [ ] Global package store + resolver + `Package.lock`
- [ ] `.bll` packing (`lang pack`) and registry protocol
- [ ] Cross-target builds

## Phase 4 — DX polish & ecosystem

- [ ] Full LSP (type-aware completion, code actions)
- [ ] Tree-sitter grammar / JetBrains (optional)
- [ ] Official registry hosting
- [ ] MCP templates and agent workflows end-to-end
- [ ] Benchmarks and performance guides

## Non-goals

- Garbage collection
- Dynamic typing / optional types that erase null safety
- Shipping dependency *source* into projects (libraries remain prebuilt `.bll`)
