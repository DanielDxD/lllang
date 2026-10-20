# ADR 006: Language Server via `tower-lsp`

**Status:** Accepted  
**Date:** 2026-03-20  
**Deciders:** LLLang core team

## Context

Editor support is mandatory for Phase 1 adoption: diagnostics, outline, folding, and syntax highlighting integration. The compiler frontend lives in Rust (`lllang-syntax`). The LSP server must share that parser, run asynchronously on stdio, and compose with other services (future index, formatter).

Alternatives considered:

| Option | Pros | Cons |
|--------|------|------|
| **tower-lsp** (Tokio) | Idiomatic async Rust, composable services, active ecosystem | Requires Tokio runtime in CLI |
| **lsp-server** (blocking) | Simple loop | Harder to extend with async index/jobs |
| Node-based LSP | Rich npm ecosystem | Duplicated grammar, drift from Rust parser |
| External Swift/Roslyn bridge | Mature analysis | Not applicable to new language |

## Decision

Implement **`lllang-lsp`** using **`tower-lsp`** on **Tokio**, launched via `lang lsp` (stdio transport).

Phase 1 feature set:

- Text document sync (incremental)
- Publish diagnostics from parse errors (`LL0xxx`)
- Document symbols and folding ranges from AST
- Semantic tokens (syntax classification)

The server **does not** embed LLVM or run codegen. It depends only on `lllang-syntax` and shared utilities (serde for LSP types).

Capability negotiation advertises Phase 1 truthfully; type-aware features unlock in later phases without breaking clients.

## Consequences

### Positive

- Single source of truth for parsing between `lang check` and IDE.
- Async pipeline ready for background indexing and `lang fmt` in Phase 2.
- Matches Rust toolchain culture (rust-analyzer-like architecture).

### Negative

- `lang` binary pulls Tokio + tower stack (acceptable for developer tool).
- Testing requires LSP JSON fixtures; mitigated by integration tests in `lllang-lsp`.

### Operational notes

- Logging via `tracing` to stderr; LSP stdout reserved for protocol.
- Crash isolation: parse panics converted to `LL6001` internal error diagnostic where possible.

## References

- [Toolchain architecture](../toolchain-architecture.md)
- [LSP (toolchain doc)](../../toolchain/lsp.md)
- [Diagnostics LL6xxx](../../spec/diagnostics.md)
- Workspace deps: `tower-lsp`, `tokio` in root `Cargo.toml`
