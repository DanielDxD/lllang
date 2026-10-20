---
name: lllang-editor-tooling
description: LLLang editor support — LSP, formatting, TextMate grammar, and VS Code/Cursor extension layout. Use when fixing diagnostics, highlighting, or IDE workflows.
---

# Editor tooling

## Language Server (Phase 1)

```bash
lang lsp    # stdio; point editors at the lang binary
```

Capabilities (syntax-focused today): parse diagnostics, document symbols, folding, semantic tokens where implemented. See [docs/toolchain/lsp.md](../../docs/toolchain/lsp.md).

## Formatter

- `lang fmt` (planned) formats `.ll` only — not `.sbl`, `.mll`, `Package.ll`, or `Solution.ll`.
- Newline-terminated style; no mandatory semicolons.

## Highlighting

- VS Code / Cursor extension under `editors/vscode`.
- Web: Shiki / highlight.js under `editors/web`.
- Grammar source of truth: [docs/spec/grammar.ebnf](../../docs/spec/grammar.ebnf), keywords in [docs/spec/keywords.md](../../docs/spec/keywords.md).

## Agent workflow

1. After editing `.ll` in this repo, run `lang check` on touched paths.
2. Keep examples parseable — Phase 1 CI may only syntax-check.
3. For LSP changes, workspace crate: `crates/lllang-lsp`; CLI: `crates/lang`.

Editor overview: [docs/toolchain/editor-support.md](../../docs/toolchain/editor-support.md).
