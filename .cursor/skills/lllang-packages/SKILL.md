---
name: lllang-packages
description: Manage LLLang dependencies, lang.toml, lockfiles, and prebuilt .bll/.sbl artifacts. Use when adding deps, publishing libraries, or explaining the package model.
---

# LLLang packages

## Model

- **No** `node_modules`-style source trees for published deps.
- Libraries ship **`.sbl`** (type surface) + per-target **`.bll`** (binaries).
- Compile against `.sbl`; link `.bll` for the active triple.
- Global store layout: `~/.lang/store/<hash>/` with `package.sbl` and `targets/*.bll`.

## Manifest essentials

```toml
[package]
name = "acme.widgets"
version = "1.0.0"
kind = "lib"          # or console/app for executables

[build]
public-api = ["src/**/*.ll"]

[dependencies]
std.io = "^0.1"
```

- Reverse-DNS package ids (`lib.geometry`, `acme.math`).
- Apps: `entry = "src/main.ll"`.
- Workspace roots use `[workspace]` with `members`.

## Publishing

- `lang pack` → registry archive; `lang publish` (Phase 3+).
- Only **public** API appears in `.sbl`; honor `[build] public-api` globs.
- ABI stability: `@abi("symbol")` on exported members; semver + abi version in `.sbl` header.

## Agent rules

- Never tell users to compile dependency source from git checkouts for normal apps.
- Commit `lang.lock` for reproducible apps/CI.
- When adding a local example library, include a checked-in `.sbl` stub if consumers typecheck against it offline.

Docs: [docs/packages/package-manager.md](../../docs/packages/package-manager.md), [docs/toolchain/manifest.md](../../docs/toolchain/manifest.md), [docs/formats/sbl.md](../../docs/formats/sbl.md).
