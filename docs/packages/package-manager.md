# Package manager

LLLang packages are resolved, cached, and linked through the **`lang`** CLI. The model deliberately avoids **`node_modules`-style source trees**: published libraries ship as **`.sbl` (types) + `.bll` (native code)** per target. Consumers typecheck against `.sbl` and link `.bll`; they do not recompile dependency `.ll` files.

## Global store

All downloaded artifacts live in a **content-addressed global store**:

```text
~/.lang/store/
  manifests/
  blobs/
    ab/
      cdef...   # .sbl, .bll, tarballs by hash
  index-cache/
```

Override root with `LANG_HOME` (default `~/.lang`).

Properties:

- **Single copy** per blob hash across projects
- **Immutable blobs** — corruption triggers re-fetch
- **No source checkout** for registry dependencies

Project-local `.lang/` holds build caches only, not a full dependency mirror.

## Resolution order

When the compiler needs package `acme.widgets`:

1. Read version from `Package.lock` (or resolve from `Package.ll` dependency requirements if lock missing)
2. Locate store entry by package id + version + content hashes
3. Load **`package.sbl`** into the module graph (**first**)
4. Select **`targets/<triple>.bll`** matching build target (**second**)
5. Verify cross-hashes between `.sbl` and `.bll` (see [`.sbl`](../formats/sbl.md))

If `.sbl` is missing, restore fails before any link attempt.

## Commands

| Command | Action |
|---------|--------|
| `lang add pkg@range` | Edit manifest, resolve, fetch, update lock |
| `lang remove pkg` | Remove dependency, prune lock |
| `lang restore` | Install lock exactly; verify hashes |
| `lang update` | Bump within semver ranges |
| `lang pack` | Produce publishable tarball from local lib |
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

Transitives are **closed over `.bll` link requirements**, not over source. Each package's manifest lists its direct runtime deps; the solver fetches a transitive closure of **prebuilt** artifacts. Static linking merges native objects; dynamic linking policy is platform-specific (future `[build] link-type`).

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
| On disk | Source + build | `.sbl` + `.bll` |
| Consumer compile | Compiles dep source | Types only from `.sbl` |
| Native code | Local codegen | Prebuilt in `.bll` |
| Per-project copy | `node_modules` | Shared store |

## Security

- Hashes in `Package.lock` pin exact blobs
- `lang publish` signs packages (planned: sigstore / minisign)
- Registry TLS required; `--insecure` disabled in release builds

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `E_SBL_BLL_HASH` | `lang restore --force` or update dependency |
| Wrong target `.bll` | `lang sdk install <triple>` + rebuild lock |
| Stale store | Remove blob path; restore re-downloads |

## See also

- [Registry protocol](registry-protocol.md)
- [Versioning & ABI](versioning-and-abi.md)
- [Manifest](../toolchain/manifest.md)
- [Solutions](../toolchain/solutions.md)
