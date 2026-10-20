---
name: lllang-cli
description: Use the lang CLI — check, build, run, new templates, Package.ll / Solution.ll paths, and phase-1 limitations. Use when scaffolding projects or documenting toolchain commands.
---

# LLLang CLI

## Phase 1 (available today)

```bash
cargo build -p lang
./target/debug/lang check <path>    # parse all .ll under path
./target/debug/lang lsp             # stdio language server
./target/debug/lang --version
```

`check` exits non-zero on parse errors; it does not typecheck or codegen yet.

## Planned workflow (document accurately)

```bash
lang new <template> -n NAME
lang new solution -n Shop            # monorepo
lang build [--release] [--project NAME] [--all]
lang run [-- args]
lang test
lang add pkg@^1.0 && lang restore
lang solution add packages/Foo
lang llm gen
```

Templates: `console`, `lib`, `cli`, `web`, `test`, `agent`, `mcp`, `bench`, `solution`.

## Manifest

- Every package has **`Package.ll`** at its root (SwiftPM-style DSL via `Std.PackageDescription`).
- Executables declare `.executable` products and `.executableTarget` entries (default layout `Sources/<Target>/`).
- Monorepos add root **`Solution.ll`** (`Std.SolutionDescription`) listing member packages.
- Lockfile: **`Package.lock`** after `lang add`; do not hand-edit.

## Conventions for agents

- Point contributors to `examples/` for copy-paste layouts; see `examples/shop-solution/` for solutions.
- Do not assume `lang build` works in Phase 1 CI unless codegen is enabled.
- Global options: `--manifest PATH` for non-default `Package.ll`; `--solution PATH` for `Solution.ll`.

Reference: [docs/toolchain/cli-reference.md](../../docs/toolchain/cli-reference.md), [docs/toolchain/templates.md](../../docs/toolchain/templates.md), [docs/toolchain/solutions.md](../../docs/toolchain/solutions.md).
