---
name: lllang-cli
description: Use the lang CLI — check, build, run, new templates, Package.ll / Solution.ll paths, and LLVM 22 requirements. Use when scaffolding projects or documenting toolchain commands.
---

# LLLang CLI

## Available today (Phase 2–4)

Requires **LLVM 22**:

```bash
brew install llvm@22
export LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22
export SDKROOT="$(xcrun --show-sdk-path)"   # macOS linking
```

```bash
cargo build -p lang
./target/debug/lang check <path>
./target/debug/lang build --manifest Package.ll
./target/debug/lang run --manifest Package.ll
./target/debug/lang fmt [--check] [PATH...]
./target/debug/lang new <template> -n NAME [-o DIR]   # console lib agent mcp bench …
./target/debug/lang new --list
./target/debug/lang pack / publish [--registry DIR] / restore / add / remove / search
./target/debug/lang test [FILTER] [--jobs N] [--sequential]
./target/debug/lang bench
./target/debug/lang sdk list|install|use
./target/debug/lang lsp   # + completion, LL2005 code actions
./target/debug/lang --version
```

Flags: `--release`, `--target`, `--emit-llvm`, `--emit-mir`, `--project`, `--all`, `--color auto|always|never`.

`LANG_REGISTRY` / `--registry` selects a file-backed registry root (default `~/.lang/registry`).

Examples: `hello`, `errors`, `shapes-oop`, `shop-solution`, `json-toon`, `async-http`, `llm-agent` (`LANG_LLM_MOCK`), `mcp-echo`, `unit-tests`.

`lang test` discovers `@suite` / `@test` / `@theory`, desugars assertion macros (`@expect`, `@equal`, `@empty`/`@notEmpty`, `@gte`/`@lte`, …), compiles a cached harness under `.lang/cache/test/`, and runs cases in parallel by default.

## Still planned / stubs

```bash
llm gen / solution add …
```

## Manifest

- **`Package.ll`** / monorepo **`Solution.ll`**; lockfile **`Package.lock`**.

Reference: [docs/toolchain/cli-reference.md](../../docs/toolchain/cli-reference.md).
