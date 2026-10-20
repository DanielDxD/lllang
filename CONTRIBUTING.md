# Contributing to LLLang

Thank you for contributing. LLLang is still early; clear specs and small, reviewable changes matter more than large dumps.

## Ways to help

1. **Specification & docs** — fix ambiguities, improve examples, keep English clear.
2. **Syntax frontend** — lexer/parser/diagnostics in `crates/lllang-syntax`.
3. **LSP** — features in `crates/lllang-lsp` (keep phase-1 scope: syntax-aware).
4. **Editors** — TextMate grammar, VS Code extension, Shiki/highlight.js.
5. **Examples & skills** — keep `examples/` parseable and skills accurate.

## Development setup

```bash
# Rust toolchain (pinned in rust-toolchain.toml)
rustup show

cargo build -p lang
cargo test
cargo fmt
cargo clippy -p lllang-syntax -p lllang-lsp -p lang -- -D warnings
```

For editor work:

```bash
cd editors/vscode && npm install
```

## Design principles

- Prefer **improving** Swift/C# ergonomics over cloning either language.
- Spec docs and `docs/spec/grammar.ebnf` are the source of truth; code and TextMate grammars must stay aligned.
- Examples under `examples/**/*.ll` must parse with `lang check`.
- Do not add LLVM codegen or a garbage collector without an ADR.

## Commit style

- Imperative subject: `Add optional chaining diagnostics`
- One logical change per commit when practical
- Update docs/skills when changing visible language behavior

## Pull requests

1. Describe *what* and *why*.
2. Link related docs or ADRs.
3. Include `lang check` / `cargo test` results for syntax changes.
4. For grammar changes, update: `grammar.ebnf`, language guide, TextMate grammar, and at least one example.

## Code of conduct

Be respectful. Assume good intent. No harassment or discrimination.
