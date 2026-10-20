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
- Keep examples under `examples/` valid against the grammar.
- Do not invent GC, `node_modules`-style dependency trees, or recompile-from-source for published libraries.
- Docs and public samples are **English**.

## Phase-1 code

- `crates/lllang-syntax` — parse only
- `crates/lllang-lsp` — syntax diagnostics, symbols, folding, semantic tokens
- `crates/lang` — CLI: `lsp`, `check`, `--version`; other commands may be stubs

When unsure, prefer updating the **specification** over inventing runtime behavior.
