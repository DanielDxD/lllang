# CLI reference (`lang`)

The **`lang`** binary is the unified entry point for project scaffolding, builds, tests, packages, documentation, LLM helpers, and the language server. Invoke `lang --help` or `lang <command> --help` for flags generated from the same source as this document.

## Global options

| Option | Description |
|--------|-------------|
| `--version` | Print compiler/toolchain version and exit |
| `--info` | Print build profile, supported targets, BLL/SBL format versions, LLVM version (when linked) |
| `-v`, `--verbose` | Increase log detail |
| `-q`, `--quiet` | Errors only |
| `--color auto\|always\|never` | Diagnostic colors |
| `--manifest PATH` | Use `Package.ll` other than cwd default |
| `--solution PATH` | Use `Solution.ll` for monorepo commands |
| `--offline` | Refuse network for package commands |

**Phase 2 implemented:** `--version`, `--info`, `check` (sema), `build`, `run`, `lsp` (syntax + sema diagnostics).

## Command overview

| Command | Status | Summary |
|---------|--------|---------|
| `new` | Planned | Scaffold from template (including `solution`) |
| `build` | **Phase 2** | Compile package or solution members |
| `solution` | Planned | Add/list/remove solution members |
| `run` | **Phase 2** | Build (if needed) and run executable |
| `watch` | Planned | Rebuild/rerun on file changes |
| `test` | Planned | Run unit/integration tests |
| `bench` | Planned | Run benchmarks |
| `check` | **Phase 2** | Parse + sema diagnostics (LL1xxx / LL2xxx) |
| `fmt` | Planned | Format source |
| `doc` | Planned | Generate documentation |
| `clean` | Planned | Remove `.lang/` artifacts |
| `add` | Planned | Add dependency |
| `remove` | Planned | Remove dependency |
| `restore` | Planned | Install from lockfile |
| `update` | Planned | Bump dependencies within constraints |
| `pack` | Planned | Produce `.sbl` + `.bll` archive |
| `publish` | Planned | Upload to registry |
| `search` | Planned | Search registry index |
| `llm` | Planned | LLM subcommands |
| `sdk` | Planned | Manage cross-compilation SDKs |
| `lsp` | **Phase 1** | Language Server (stdio) |

---

## `lang new`

Create a new project from a template.

```bash
lang new <template> -n <name> [--path DIR]
```

| Template | Output |
|----------|--------|
| `console` | Executable app |
| `lib` | Library package |
| `cli` | CLI with argument parsing |
| `web` | Web-oriented app skeleton |
| `test` | Test-focused layout |
| `agent` | Agent/automation project |
| `mcp` | MCP server template |
| `bench` | Benchmark harness |
| `solution` | Monorepo root (`Solution.ll`) |

See [Templates](templates.md) for directory trees.

---

## `lang new solution`

Scaffold a monorepo with `Solution.ll`, starter `packages/`, and `apps/`:

```bash
lang new solution -n shop [--path DIR]
```

See [Solutions](solutions.md) and the `solution` template in [Templates](templates.md).

---

## `lang build`

```bash
lang build [--release] [--target TRIPLE] [--product NAME] [--project NAME] [--all] [-w]
```

- Compiles the current package, or solution `defaultProjects` when `Solution.ll` is present
- `--project NAME` builds one member (and its path dependencies)
- `--all` builds every project listed in `Solution.ll`
- Libraries emit `.bll` under `targets/` when `--pack` is combined (see `pack`)
- `-w` treats warnings as errors

---

## `lang run`

```bash
lang run [--release] [--] [ARGS...]
```

Requires an executable **product** in `Package.ll`. Forwards args to `main`.

---

## `lang watch`

```bash
lang watch <build|run|test> [flags...]
```

File watcher with incremental rebuild. Debounce configurable via `[build] watch-debounce-ms`.

---

## `lang test`

```bash
lang test [FILTER] [--coverage] [--parallel N]
```

Discovers tests in `tests/` and inline `@Test` functions. See [Testing](testing.md).

---

## `lang bench`

```bash
lang bench [FILTER] [--compare BASELINE]
```

Runs `benches/` and prints statistical summary.

---

## `lang check`

**Phase 2 — available.**

```bash
lang check [PATH...]
```

Recursively parse `.ll` files and run semantic analysis. Exit code 0 iff no errors.

```bash
lang check .
lang check src/foo.ll
```

---

## `lang fmt`

```bash
lang fmt [PATH...] [--check] [--write]
```

Idempotent formatter; `--check` for CI. See [Formatter](formatter.md).

---

## `lang doc`

```bash
lang doc [--open] [--output DIR]
```

Extracts `///` comments to static HTML (or Markdown). Uses public API same as `.sbl` export.

---

## `lang clean`

```bash
lang clean [--deep]
```

Removes `.lang/build`, incremental caches. `--deep` also clears fetched pre-release packages in local cache (not global store).

---

## Package commands

### `lang add`

```bash
lang add <package>[@range] [...]
lang add --dev test.utils@^1.0
```

Updates `Package.ll` dependencies and `Package.lock`, fetches into global store.

### `lang remove`

```bash
lang remove <package> [...]
```

### `lang restore`

```bash
lang restore [--force]
```

Installs exact versions from `Package.lock`; verifies `.sbl`/`.bll` hashes.

### `lang update`

```bash
lang update [package...]
```

Resolves newest versions allowed by semver ranges.

### `lang pack`

```bash
lang pack [--target TRIPLE] [--output FILE]
```

Builds release artifacts and writes registry tarball (`.sbl` + per-target `.bll` + manifest).

### `lang publish`

```bash
lang publish [--registry URL] [--dry-run]
```

Requires auth token (`LANG_REGISTRY_TOKEN` or credential helper).

### `lang search`

```bash
lang search QUERY [--limit N]
```

Queries registry HTTP index. See [Registry protocol](../packages/registry-protocol.md).

---

## `lang llm`

| Subcommand | Description |
|------------|-------------|
| `gen` | Generate or update `.mll` from sources |
| `context` | Emit TOON/JSON context bundle for agents |
| `explain` | Short natural-language summary of a symbol or module (local heuristic + optional model hook) |

```bash
lang llm gen [-p PACKAGE]
lang llm context [--max-tokens N] [--include-sources]
lang llm explain Acme.Widgets.Button
```

---

## `lang sdk`

```bash
lang sdk list
lang sdk install <target-triple>
lang sdk remove <target-triple>
```

Manages LLVM/sysroot bundles for cross-compilation. Configured via `Package.ll` `settings:` / environment (see [Manifest](manifest.md)).

---

## `lang solution`

Manage members of a `Solution.ll` monorepo (planned):

```bash
lang solution list
lang solution add <path>
lang solution remove <name>
```

`lang new lib -n Foo --path packages/Foo --solution .` creates a package and registers it in the solution.

---

## `lang lsp`

**Phase 1 — available.**

```bash
lang lsp
```

Starts LSP over stdio (JSON-RPC). Editors spawn this process. Capabilities: [LSP](lsp.md).

No flags in Phase 1; configuration via editor client.

---

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General failure |
| 2 | Invalid usage |
| 3 | Diagnostic errors (check/build/test) |
| 4 | Network/registry failure |

---

## Configuration precedence

1. CLI flags
2. Environment (`LANG_*`)
3. `Package.ll` / `Solution.ll` evaluated settings
4. User config `~/.lang/config.toml` (planned)
5. Toolchain defaults

---

## Phase roadmap

| Phase | Commands |
|-------|----------|
| **1** | `lsp` (syntax), `check` (parse), `--version`, `--info` |
| **2 (current)** | `check` (sema), `build`, `run`, `lsp` (sema diags) |
| **2.1** | `fmt`, go-to-def / rename in LSP |
| **3** | `add`, `restore`, `pack`, `publish`, `search`, full package graph |
| **4** | `watch`, `bench`, `doc`, `llm`, `sdk` polish |

Stubs may print guidance pointing to this page and [ROADMAP](../../ROADMAP.md).

## See also

- [Manifest](manifest.md)
- [Solutions](solutions.md)
- [Build pipeline](build-pipeline.md)
- [Installation](../getting-started/installation.md)
