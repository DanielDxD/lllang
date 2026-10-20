---
name: lllang-cli
description: Use the lang CLI — check, build, run, new templates, Package.ll / Solution.ll paths, and LLVM 22 requirements. Use when scaffolding projects or documenting toolchain commands.
---

# LLLang CLI

## Phase 2 (available today)

Requires **LLVM 22**:

```bash
brew install llvm@22
export LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22
export SDKROOT="$(xcrun --show-sdk-path)"   # macOS linking
```

```bash
cargo build -p lang
./target/debug/lang check <path>              # parse + sema
./target/debug/lang build --manifest Package.ll
./target/debug/lang run --manifest Package.ll
./target/debug/lang run --solution Solution.ll --project Cli
./target/debug/lang lsp                       # stdio; syntax + sema diagnostics
./target/debug/lang --version
```

Flags: `--release`, `--emit-llvm`, `--emit-mir`, `--project`, `--all`.

Known-good examples: `examples/hello`, `errors`, `shapes-oop`, `shop-solution`.

## Still planned / stubs

```bash
lang new <template> -n NAME
lang fmt
lang test / add / publish / llm …
```

## Manifest

- Every package has **`Package.ll`** at its root (SwiftPM-style DSL via `Std.PackageDescription`).
- Executables declare `.executable` products and `.executableTarget` entries.
- Monorepos add root **`Solution.ll`** (`Std.SolutionDescription`) listing member packages.
- Lockfile: **`Package.lock`** after `lang add`; do not hand-edit.

## Conventions for agents

- Point contributors to `examples/` for copy-paste layouts; see `examples/shop-solution/` for solutions.
- Global options: `--manifest PATH` for `Package.ll`; `--solution PATH` for `Solution.ll`.

Reference: [docs/toolchain/cli-reference.md](../../docs/toolchain/cli-reference.md), [docs/getting-started/installation.md](../../docs/getting-started/installation.md).
