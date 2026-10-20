# Installation

LLLang ships as a single **`lang`** CLI binary plus optional editor extensions. Phase 2 adds semantic checking and native `build`/`run` via LLVM 22 (see [ROADMAP](../../ROADMAP.md)).

## Requirements

| Component | Notes |
|-----------|-------|
| OS | macOS, Linux, Windows |
| CPU | x64 or arm64 |
| Rust | 1.85+ (to build from source) |
| LLVM | **LLVM 22** required to compile `lllang-llvm` / run `lang build` |

You do **not** need Node.js, a JVM, or a separate package runtime. Dependencies are prebuilt `.bll` libraries resolved into `~/.lang/store` (see [Package manager](../packages/package-manager.md)).

### LLVM 22 (macOS Homebrew)

```bash
brew install llvm@22
export LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22
export PATH="/opt/homebrew/opt/llvm@22/bin:$PATH"
# Optional but recommended when linking with Apple clang:
export SDKROOT="$(xcrun --show-sdk-path)"
```

If `LLVM_SYS_221_PREFIX` is unset, `lllang-llvm`'s `build.rs` fails with a clear message. Integration tests that need codegen skip automatically when LLVM is missing.

## Install from source

Clone the repository and build the workspace:

```bash
git clone https://github.com/lllang-lang/llang.git
cd llang
rustup toolchain install stable
export LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22   # see above
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

Commands that work in Phase 2:

```bash
lang check path/to/file.ll
lang build --manifest examples/hello/Package.ll
lang run --manifest examples/hello/Package.ll
lang lsp          # stdio JSON-RPC; syntax + sema diagnostics
```

`lang fmt` and package-registry commands (`add`, `publish`, …) remain stubs until Phase 2.1 / 3.

## Prebuilt releases (planned)

Official releases will publish:

- **`lang`** — CLI and LSP in one binary
- **VS Code / Cursor extension** — grammar + LSP client ([Editor support](../toolchain/editor-support.md))
- Optional **SDK bundles** for cross-compilation targets (`lang sdk install …`)

Download from GitHub Releases, extract, and place `lang` on your `PATH`. Release archives will include a `SHA256SUMS` file.

## Editor setup

1. Install the **LLLang** extension from the marketplace (or load `editors/vscode` in development mode).
2. Point the extension at your `lang` binary, or rely on `PATH`.
3. Open a folder containing `Package.ll` (or a monorepo root with `Solution.ll`) so the LSP activates.

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
- [Project anatomy](project-anatomy.md) — `Package.ll`, sources, and lockfile
- [CLI reference](../toolchain/cli-reference.md) — full command surface
