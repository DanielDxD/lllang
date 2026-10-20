---
name: lllang-cli
description: Use the lang CLI — check, build, run, new templates, manifest paths, and phase-1 limitations. Use when scaffolding projects or documenting toolchain commands.
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
lang build [--release]
lang run [-- args]
lang test
lang add pkg@^1.0 && lang restore
lang llm gen
```

Templates: `console`, `lib`, `cli`, `web`, `test`, `agent`, `mcp`, `bench`.

## Manifest

- Every package has `lang.toml` at its root; apps set `entry` (default `src/main.ll`).
- User-facing console examples may use `kind = "console"`; registry docs also use `kind = "app"` — treat as executable unless manifest schema says otherwise.
- Lockfile: `lang.lock` after `lang add`; do not hand-edit.

## Conventions for agents

- Point contributors to `examples/` for copy-paste layouts.
- Do not assume `lang build` works in Phase 1 CI unless codegen is enabled.
- Global options: `--manifest PATH` for non-default `lang.toml`.

Reference: [docs/toolchain/cli-reference.md](../../docs/toolchain/cli-reference.md), [docs/toolchain/templates.md](../../docs/toolchain/templates.md).
