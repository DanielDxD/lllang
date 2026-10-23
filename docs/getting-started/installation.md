# Installation

LLLang ships as a single **`lang`** CLI binary. End users download a prebuilt release; **LLVM 22 is only required when building the compiler from source** (contributors / CI builders).

## Quick install (recommended)

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.sh | bash
```

Options:

```bash
curl -fsSL …/install.sh | bash -s -- --version 0.2.0
curl -fsSL …/install.sh | bash -s -- --prefix "$HOME/.lang/toolchain" --no-modify-path
curl -fsSL …/install.sh | bash -s -- --skip-deps
```

Default prefix: `~/.lang/toolchain`. The script downloads the correct GitHub Release asset for your host triple, verifies `SHA256SUMS`, installs host link deps when possible (Xcode CLT / `clang` + `libsqlite3`), and appends `bin/` to your shell `PATH`.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.ps1 | iex
```

Options:

```powershell
irm …/install.ps1 | iex   # latest
.\install.ps1 -Version 0.2.0 -Prefix "$env:LOCALAPPDATA\lang\toolchain"
.\install.ps1 -NoModifyPath -SkipDeps
```

Default prefix: `%LOCALAPPDATA%\lang\toolchain`. Installs Clang via `winget install LLVM.LLVM` unless `-SkipDeps` is set. Ships `sqlite3.dll` with the toolchain.

Verify:

```bash
lang --version
lang --info
```

## Release assets

| Triple | Archive | Runner |
|--------|---------|--------|
| `aarch64-apple-darwin` | `lang-<ver>-aarch64-apple-darwin.tar.gz` | macOS Apple Silicon |
| `x86_64-apple-darwin` | `lang-<ver>-x86_64-apple-darwin.tar.gz` | macOS Intel |
| `x86_64-unknown-linux-gnu` | `lang-<ver>-x86_64-unknown-linux-gnu.tar.gz` | Linux x64 |
| `aarch64-unknown-linux-gnu` | `lang-<ver>-aarch64-unknown-linux-gnu.tar.gz` | Linux arm64 |
| `x86_64-pc-windows-msvc` | `lang-<ver>-x86_64-pc-windows-msvc.zip` | Windows x64 |

Each GitHub Release also publishes a combined **`SHA256SUMS`** file.

### Archive layout

```
bin/lang[.exe]     CLI (+ LSP via `lang lsp`)
lib/<triple>/      Prebuilt C runtime for that LLVM triple (`liblllang_rt.a` / `lllang_rt.lib`)
lib/               Legacy flat copy of the host runtime (still resolved as fallback)
runtime/           Runtime sources (+ `windows/` sqlite on Windows zips)
docs/              Install notes
```

## End-user host dependencies

| Platform | Needed for `lang build` / `lang run` |
|----------|--------------------------------------|
| macOS | Xcode Command Line Tools (clang + SDK); system sqlite |
| Linux | `clang`, `libsqlite3` (e.g. `apt install clang libsqlite3-dev`) |
| Windows | Clang (`winget install LLVM.LLVM`); sqlite DLL shipped in the zip |

You do **not** need Node.js, a JVM, or LLVM 22 on end-user machines.

For HTTPS in v0.2, terminate TLS at a reverse proxy (Caddy, nginx, cloud LB) in front of `HttpServer.serve` — native TLS is not in the freeze yet ([SHIPPED.md](../stdlib/SHIPPED.md)).

## Contributors: build from source

Requires [Rust](https://rustup.rs/) 1.85+ and **LLVM 22**.

### macOS (Homebrew)

```bash
brew install llvm@22
export LLVM_SYS_221_PREFIX=$(brew --prefix llvm@22)
export PATH="$LLVM_SYS_221_PREFIX/bin:$PATH"
# Optional when linking with Apple clang:
export SDKROOT="$(xcrun --show-sdk-path)"
```

### Linux

Install LLVM 22 (distro package or [apt.llvm.org](https://apt.llvm.org/)) and set `LLVM_SYS_221_PREFIX` (often `/usr/lib/llvm-22`). Also install `clang` and `libsqlite3-dev`.

### Windows

```powershell
winget install --id LLVM.LLVM -e
$env:LLVM_SYS_221_PREFIX = "C:\Program Files\LLVM"
$env:Path = "$env:LLVM_SYS_221_PREFIX\bin;$env:Path"
pwsh ./scripts/fetch-sqlite-windows.ps1
$env:CC = "clang"
```

### Build

```bash
git clone https://github.com/lllang-lang/llang.git
cd llang
cargo build -p lang --release
export PATH="$PWD/target/release:$PATH"
lang --version
```

Pack a local release archive:

```bash
# Unix
./scripts/pack-release.sh
# → dist/lang-<version>-<triple>.tar.gz + SHA256SUMS

# Windows
pwsh ./scripts/pack-release.ps1
# → dist/lang-<version>-x86_64-pc-windows-msvc.zip
```

## Editor setup

1. Install the **LLLang** extension from the marketplace (or load `editors/vscode` in development mode).
2. Point the extension at your `lang` binary, or rely on `PATH`.
3. Open a folder containing `Package.ll` (or a monorepo root with `Solution.ll`) so the LSP activates.

Web sites can use Shiki or highlight.js grammars from `editors/web` — see [Editor support](../toolchain/editor-support.md).

## Environment variables

| Variable | Purpose |
|----------|---------|
| `LANG_HOME` | Override global data dir (default `~/.lang` / `%LOCALAPPDATA%\lang`) |
| `LANG_RUNTIME_LIB` | Absolute path to `liblllang_rt.a` / `lllang_rt.lib` |
| `LANG_OS` / `LANG_ARCH` | Go-like default target when `--target` omitted (`linux`/`windows`/`darwin` + `amd64`/`arm64`) |
| `LANG_SQLITE_DIR` | Windows: root containing `lib/` + `bin/sqlite3.dll` |
| `LANG_REGISTRY` | Default registry URL for `lang add` / `publish` |
| `LANG_LOG` | `debug`, `info`, `warn`, `error` for CLI/LSP logging |
| `LLVM_SYS_221_PREFIX` | Contributors only: LLVM 22 install prefix |
| `RUST_BACKTRACE` | Rust backtraces when developing the compiler |

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `LLVM 22 not found` when building `lang` | Set `LLVM_SYS_221_PREFIX` (contributors only) |
| `link failed` / missing sqlite | Install host sqlite (Unix) or re-run the Windows installer / `fetch-sqlite-windows.ps1` |
| `runtime archive not found` | Reinstall the toolchain so `lib/` exists next to `bin/`, or set `LANG_RUNTIME_LIB` |
| `lang: command not found` | Ensure `…/toolchain/bin` is on `PATH` (new shell after install) |
