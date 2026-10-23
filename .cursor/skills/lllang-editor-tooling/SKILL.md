---
name: lllang-editor-tooling
description: LLLang editor support — LSP, formatting, TextMate grammar, and VS Code/Cursor extension layout. Use when fixing diagnostics, highlighting, or IDE workflows.
---

# Editor tooling

## Language Server

```bash
lang lsp    # stdio; point editors at the lang binary
```

Capabilities: parse + sema diagnostics, document symbols, folding, go-to-definition, references, rename, inlay hints, typed hover (incl. `throws` / `#deprecated`), document formatting, type-aware completion (members after `.`, including extensions), code actions for missing `try` (LL2005), `override` (LL2000), cast kind (LL2002), empty-array annotation (LL3001). See [docs/toolchain/lsp.md](../../docs/toolchain/lsp.md).

Tree-sitter stub: [`editors/tree-sitter-lllang`](../../editors/tree-sitter-lllang) (experimental; covers extensions / `#defer` / actors lightly; TextMate remains primary).

## Formatter

- `lang fmt [--check] [PATH...]` formats `.ll` source (4-space indent, soft wrap 100, sorted imports).
- Does not rewrite `.sbl`, `.mll`, or lockfiles; `Package.ll` / `Solution.ll` are `.ll` DSL and format when included.
- LSP `documentFormattingProvider` uses the same pretty-printer.

## Highlighting

- VS Code / Cursor extension under `editors/vscode`.
- Web: Shiki / highlight.js under `editors/web`.
- Grammar source of truth: [docs/spec/grammar.ebnf](../../docs/spec/grammar.ebnf), keywords in [docs/spec/keywords.md](../../docs/spec/keywords.md).

## Agent workflow

1. After editing `.ll` in this repo, run `lang check` / `lang fmt --check` on touched paths.
2. Keep examples buildable — `cargo test -p lang --test build_examples`.
3. For LSP changes, workspace crate: `crates/lllang-lsp`; CLI: `crates/lang`.

Editor overview: [docs/toolchain/editor-support.md](../../docs/toolchain/editor-support.md).
