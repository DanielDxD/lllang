# Versioning and ABI

LLLang separates **package semver** (API/evolution for humans and the resolver) from **ABI epoch** (binary compatibility for `.bll` / `.sbl` linking). Getting both right keeps the global store and registries safe across compiler upgrades.

## Package versioning (semver)

Published packages use **semantic versioning** in registry metadata (resolved from `Package.ll` dependency requirements) and registries:

| Bump | When |
|------|------|
| **MAJOR** | Breaking source API, breaking `@abi` layout, ABI epoch change |
| **MINOR** | Additive API (new types, methods with defaults) |
| **PATCH** | Bug fixes, performance, no API/ABI surface change |

Version requirements in `Package.ll`:

```ll
dependencies: [
    .package(id: "acme.math", range: "^1.4"),
    .package(id: "std.json", range: "~0.2.1"),
]
```

`Package.lock` pins exact versions and content hashes.

### Source compatibility

Consumers may compile against newer **minor** releases if `.sbl` only adds symbols. Removing or changing signatures requires **major** bump.

## ABI epoch

The **ABI version** is an integer in `.sbl` (`abi 2`) and `.bll` headers. It increments when:

- Calling convention or name mangling changes
- `@abi` symbol layout changes for existing names
- Runtime object layout for exported types changes
- ARC/runtime helper contracts change

Compiler `lang --info` reports supported ABI epochs. A consumer compiler rejects `.bll` built for a different epoch even at the same package semver.

| Epoch change | Package semver |
|--------------|----------------|
| ABI epoch +1 | Must major bump |
| Add `@abi` export | Minor |
| Fix internal `.bll` bug, same symbols | Patch |

## `@abi` stability

Public native symbols use `@abi("stable_name")` in `.sbl`. Rules:

1. **Never rename** without major + ABI epoch bump
2. **Never change** parameter types or return kind for same ABI name
3. **Deprecate** in source docs first; keep symbol until major release
4. Generic exports must be **monomorphized** in the published `.bll` or explicitly listed in bitcode policy

Link failures cite package, version, and symbol.

## Hash binding

Each publish produces paired digests:

- **SBL hash** — canonical declaration text
- **BLL hash** — entire portable `.bll` blob (one per package version)

Cross-embedded hashes ensure:

- Registry cannot swap `.bll` without matching `.sbl`
- Local store corruption is detected at restore

See [`.sbl`](../formats/sbl.md) and [`.bll`](../formats/bll.md).

## Compiler vs package version

| Artifact | Versioning |
|----------|------------|
| `lang` CLI / compiler | Toolchain semver (e.g. `0.1.0`) |
| `std.*` packages | Independent semver, tied to compiler in release notes |
| Third-party libs | Independent semver |

**Minimum compiler** field (planned in manifest):

```toml
[package]
name = "acme.widgets"
version = "2.0.0"
min-lang = "0.3.0"
```

Resolver warns when `lang --version` is too old.

## Yanking and security

- **Yank** a bad release at registry (`yanked: true`) without deleting blobs
- **Patch** release with same API/ABI replaces bug; new hash in lockfile
- Critical ABI break: major bump + new epoch; old blobs remain for lockfiles

## Upgrade strategies for library authors

1. Ship additive minor with new optional APIs
2. Deprecate via docs and analyzer warnings (Phase 2+)
3. Remove in next major with migration guide in `.mll` `pitfalls`

## Upgrade strategies for app authors

```bash
lang update              # respect semver ranges
lang update acme.math    # single package
```

Review lockfile diff in CI; run tests after hash changes.

## Compatibility matrix (example)

| Compiler | ABI epoch | Reads `.bll` epoch |
|----------|-----------|-------------------|
| 0.1.x | 1 | 1 |
| 0.2.x | 2 | 1–2 (compat mode) |
| 0.3.x | 2 | 2 only |

Exact matrix ships with each release.

## See also

- [Package manager](package-manager.md)
- [Registry protocol](registry-protocol.md)
- [Manifest](../toolchain/manifest.md)
