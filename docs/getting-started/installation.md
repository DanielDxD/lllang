# Installation

LLLang ships as a single **`lang`** CLI binary plus optional editor extensions. Phase 1 focuses on syntax checking and the language server; full compilation and package publishing arrive in later phases (see [ROADMAP](../../ROADMAP.md)).

## Requirements

| Component | Phase 1 | Full toolchain (Phase 2+) |
|-----------|---------|---------------------------|
| OS | macOS, Linux, Windows | Same |
| CPU | x64 or arm64 | Same |
| Rust | 1.85+ (to build from source) | Same |
| LLVM | — | LLVM 22 (bundled with releases) |

You do **not** need Node.js, a JVM, or a separate package runtime. Dependencies are prebuilt `.bll` libraries resolved into `~/.lang/store` (see [Package manager](../packages/package-manager.md)).

## Install from source (recommended in Phase 1)

Clone the repository and build the workspace:

```bash
git clone https://github.com/lllang-lang/llang.git
cd llang
rustup toolchain install stable
cargo build -p lang --release
```

Add the binary to your `PATH`:

```bash
export PATH="$PWD/target/release:$PATH"
# or: cargo install --path crates/lang
```

Verify:

```bash
lang --version
lang --info
```

Expected Phase 1 commands that work today:

```bash
lang check path/to/project
lang lsp          # stdio JSON-RPC; used by editors
```

Other subcommands (`build`, `run`, `add`, …) may print “not yet implemented” until Phase 2–3.

## Prebuilt releases (planned)

Official releases will publish:

- **`lang`** — CLI and LSP in one binary
- **VS Code / Cursor extension** — grammar + LSP client ([Editor support](../toolchain/editor-support.md))
- Optional **SDK bundles** for cross-compilation targets (`lang sdk install …`)

Download from GitHub Releases, extract, and place `lang` on your `PATH`. Release archives will include a `SHA256SUMS` file.

## Editor setup

1. Install the **LLLang** extension from the marketplace (or load `editors/vscode` in development mode).
2. Point the extension at your `lang` binary, or rely on `PATH`.
3. Open a folder containing `lang.toml` so the LSP activates.

Web sites can use Shiki or highlight.js grammars from `editors/web` — see [Editor support](../toolchain/editor-support.md).

## Environment variables

| Variable | Purpose |
|----------|---------|
| `LANG_HOME` | Override global data dir (default `~/.lang`) |
| `LANG_REGISTRY` | Default registry URL for `lang add` / `publish` |
| `LANG_LOG` | `debug`, `info`, `warn`, `error` for CLI/LSP logging |
| `RUST_BACKTRACE` | Rust backtraces when developing the compiler |

## Troubleshooting

**`lang: command not found`** — Ensure `target/release` or `cargo install` destination is on `PATH`.

**LSP does not start in the editor** — Run `lang lsp` manually; it should block on stdin. Check editor output panel for the path to the binary.

**`check` reports parse errors on valid-looking code** — Phase 1 validates syntax only; semantic errors appear in Phase 2. Compare with [grammar](../spec/grammar.ebnf) and [lexical structure](../language/01-lexical-structure.md).

## Next steps

- [Hello, world](hello-world.md) — create a project and run checks
- [Project anatomy](project-anatomy.md) — `lang.toml`, sources, and lockfile
- [CLI reference](../toolchain/cli-reference.md) — full command surface
