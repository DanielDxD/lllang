# Package manager

LLLang packages are resolved, cached, and linked through the **`lang`** CLI. The model deliberately avoids **`node_modules`-style source trees**: published libraries ship as **`.sbl` (types) + portable `.bll` (LLVM bitcode)**. Consumers typecheck against `.sbl` and **finalize** `.bll` for the active `--target`; they do not recompile dependency `.ll` files.

See [ADR 010](../design/adr/010-portable-bll-multitarget.md).

## Global store

All downloaded artifacts live in a **content-addressed global store**:

```text
~/.lang/store/
  manifests/
  blobs/
    ab/
      cdef...   # .sbl, .bll, tarballs by hash
  packages/<name>/<version>/
    package.sbl
    package.bll   # portable BLL v2 (not per-triple)
  index-cache/
```

Override root with `LANG_HOME` (default `~/.lang`).

Properties:

- **Single copy** per blob hash across projects
- **Immutable blobs** — corruption triggers re-fetch
- **No source checkout** for registry dependencies
- **One `.bll` per package version** (portable bitcode)

Project-local `.lang/` holds build caches (including materialized objects per triple), not a full dependency mirror.

## Resolution order

When the compiler needs package `acme.widgets`:

1. Read version from `Package.lock` (or resolve from `Package.ll` dependency requirements if lock missing)
2. Locate store entry by package id + version + content hashes
3. Load **`package.sbl`** into the module graph (**first**)
4. Load **`package.bll`** (portable) and materialize to a native object for the **build target** (**second**)
5. Verify cross-hashes between `.sbl` and `.bll` (see [`.sbl`](../formats/sbl.md))

If `.sbl` is missing, restore fails before any link attempt.

## Multi-target builds

```bash
lang build --target linux/amd64
# or
LANG_OS=windows LANG_ARCH=amd64 lang build
```

Official aliases: `darwin/arm64` / `macos/arm64`, `linux/amd64`, `linux/arm64`, `windows/amd64`, `windows/arm64`.

Runtime archives live under `$LANG_HOME/toolchain/lib/<triple>/`. Cross-link may require a sysroot (`lang sdk install <triple>`).

## Commands

| Command | Action |
|---------|--------|
| `lang add pkg@range` | Edit manifest, resolve, fetch, update lock |
| `lang remove pkg` | Remove dependency, prune lock |
| `lang restore` | Install lock exactly; verify hashes |
| `lang update` | Bump within semver ranges |
| `lang pack` | Produce portable `.sbl` + `.bll` |
| `lang publish` | Upload to registry |

See [CLI reference](../toolchain/cli-reference.md).

## Dependency kinds

Runtime dependencies live in `Package.ll` `dependencies:`:

```ll
dependencies: [
    .package(id: "acme.math", from: "1.0.0"),
    .package(path: "../LocalKit"),
]
```

Dev-only dependencies attach on **test targets** (`.product(name:package:)` or registry ids), not in a separate manifest table. They never appear in published library metadata or consumer lockfiles for those consumers.

## Transitive dependencies

Transitives are **closed over `.bll` link requirements**, not over source. Each package's manifest lists its direct runtime deps; the solver fetches a transitive closure of **prebuilt** artifacts. At link time, portable bitcode is finalized for the active triple; static linking merges the resulting objects.

## Offline and mirrors

```bash
lang restore --offline
```

Uses store only. CI caches `~/.lang/store` keyed by `Package.lock` hash.

Corporate mirrors set:

```bash
export LANG_REGISTRY=https://registry.internal/lang/
```

## Contrast with other ecosystems

| | npm/cargo source deps | LLLang |
|---|----------------------|--------|
| On disk | Source + build | `.sbl` + portable `.bll` |
| Consumer compile | Compiles dep source | Types from `.sbl`; finalize bitcode for `--target` |
| Native code | Local codegen from source | Materialized from bitcode at link |
| Per-project copy | `node_modules` | Shared store |

## Security

- Hashes in `Package.lock` pin exact blobs (one `bll` hash per package)
- `lang publish` signs packages (planned: sigstore / minisign)
- Registry TLS required; `--insecure` disabled in release builds

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `E_SBL_BLL_HASH` | `lang restore --force` or update dependency |
| BLL v1 / native-only blob | Republish with `lang pack` (BLL v2 portable) |
| Cross-link missing runtime | Install runtime under `toolchain/lib/<triple>/` or `lang sdk install <triple>` |
| Stale store | Remove blob path; restore re-downloads |

## See also

- [Registry protocol](registry-protocol.md)
- [Versioning & ABI](versioning-and-abi.md)
- [Manifest](../toolchain/manifest.md)
- [Solutions](../toolchain/solutions.md)
