# Project anatomy

An LLLang **package** is a directory with a `lang.toml` manifest, source under `src/`, optional tests, and (for applications) a defined entry point. Workspaces can group multiple packages. This page maps folders and files to toolchain behavior.

## Top-level layout

```text
my-app/
  lang.toml              # required manifest
  lang.lock              # generated lockfile (after restore/add)
  .lang/                 # local cache (gitignored); build artifacts
  src/                   # .ll source
  tests/                 # test sources (lang test)
  benches/               # benchmarks (lang bench)
  docs/                  # optional human docs (lang doc may include)
  assets/                # bundled resources (referenced from manifest)
  skills/                # optional agent skill packs (LLM workflows)
```

Libraries and apps share the same shape; only `[package]` `kind` and entry configuration differ.

## `lang.toml`

The manifest drives name, version, dependencies, build settings, and SDK pins. Full reference: [Manifest](../toolchain/manifest.md).

Minimal executable:

```toml
[package]
name = "my-app"
version = "0.1.0"
kind = "app"
entry = "src/main.ll"

[build]
target = "native"
```

Minimal library (published as `.sbl` + per-target `.bll`):

```toml
[package]
name = "acme.widgets"
version = "1.0.0"
kind = "lib"

[build]
public-api = "src/**/*.ll"   # globs exported to .sbl
```

## Source tree (`src/`)

| Pattern | Role |
|---------|------|
| `src/main.ll` | Default entry for `kind = "app"` |
| `src/lib.ll` or module files | Library root modules |
| `src/**/*.ll` | Additional modules; namespaces mirror folder layout by convention |

**Namespaces** in source (`namespace Acme.Widgets`) align with package name segments but are not required to match filesystem paths — the compiler resolves modules via `import` and manifest roots.

Phase 1: only parsing and `lang check`. Phase 2+: module graph, typecheck, codegen.

## Tests (`tests/`)

Test projects use the **test** template or a `tests/` folder:

```text
tests/
  widgets_test.ll
```

`lang test` discovers tests marked with attributes or naming conventions (see [Testing](../toolchain/testing.md)). Tests compile against the same dependency graph as the main package.

## Lockfile (`lang.lock`)

After `lang add`, `lang update`, or `lang restore`, the lockfile records:

- Exact dependency versions
- Content hashes of `.bll` / `.sbl` blobs
- Target-specific `.bll` selections

Commit `lang.lock` for applications and binaries; libraries may commit it for reproducible CI. Never hand-edit.

## Local cache (`.lang/`)

Build outputs, incremental IR, and fetched package extractions may live under `.lang/`. Safe to delete; `lang clean` removes it. Not published to registries.

## Dependencies on disk

There is **no** `node_modules` tree of source. Resolved packages appear as:

```text
~/.lang/store/<hash>/...
  manifest.toml
  package.sbl
  targets/
    aarch64-apple-darwin.bll
    x86_64-unknown-linux-gnu.bll
```

The compiler reads `.sbl` for types and links `.bll` for the active target. See [Package manager](../packages/package-manager.md).

## Workspace (monorepo)

```toml
[workspace]
members = ["packages/*", "apps/cli"]
default-member = "apps/cli"
```

`lang build` at the workspace root builds the default member or all members with `--workspace`. Shared `[workspace.dependencies]` pin versions across members.

## Artifacts you ship

| Kind | Ship to registry | Ship to end users |
|------|------------------|-------------------|
| App | — | Native binary (or bundle) |
| Library | `.sbl` + `.bll` per target | Same (via dependency) |
| MCP / agent template | Package + `.mll` | Template + metadata |

`lang pack` produces a registry-ready archive; `lang publish` uploads it (Phase 3+).

## `.gitignore` recommendations

```gitignore
.lang/
target/           # if using cargo adjacency for native shims
*.bll             # except checked-in examples
lang.lock         # only if you intentionally float deps (unusual)
```

## Related docs

- [Manifest](../toolchain/manifest.md)
- [Templates](../toolchain/templates.md)
- [Build pipeline](../toolchain/build-pipeline.md)
- [`.ll` format](../formats/ll.md)
