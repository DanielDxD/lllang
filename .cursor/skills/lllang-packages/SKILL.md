---
name: lllang-packages
description: Manage LLLang dependencies, Package.ll, Solution.ll, lockfiles, and prebuilt .bll/.sbl artifacts. Use when adding deps, publishing libraries, or explaining the package model.
---

# LLLang packages

## Model

- **No** `node_modules`-style source trees for published deps.
- Libraries ship **`.sbl`** (type surface) + **one portable `.bll`** (BLL v2 LLVM bitcode).
- Compile against `.sbl`; **finalize** `.bll` → native object for `--target` at link.
- Store: `~/.lang/store/packages/<name>/<ver>/{package.sbl,package.bll}`.
- Official targets: `darwin/arm64`, `linux/amd64`, `linux/arm64`, `windows/amd64`, `windows/arm64`.

## CLI

```bash
lang pack                         # emit portable package.sbl + package.bll
lang publish [--registry DIR]     # pack + file registry (~/.lang/registry or LANG_REGISTRY)
lang restore                      # store + fetch from file registry when needed
lang add <id> / lang remove <id>  # pin path or registry-id local entries
lang search <query> [--registry]  # registry index, else store
lang sdk list|install|use <triple>
lang build --target linux/amd64   # Go-like alias or full triple; LANG_OS + LANG_ARCH
```


## Manifest essentials

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "acme.widgets",
    products: [
        .library(name: "Widgets", targets: ["Widgets"]),
    ],
    dependencies: [
        .package(id: "std.io", from: "0.1.0"),
    ],
    targets: [
        .target(name: "Widgets", path: "Sources/Widgets"),
        .testTarget(name: "WidgetsTests", dependencies: ["Widgets"], path: "Tests/WidgetsTests"),
    ],
)
```

- Reverse-DNS package ids (`lib.geometry`, `acme.math`).
- Apps: `.executable` product + `.executableTarget`.
- Monorepos: root **`Solution.ll`** + path deps (`.package(path: "../Core")`); no TOML `[workspace]`.

## Publishing

- `lang pack` → store archive; `lang publish` still planned for official registry hosting.
- Only **public** API appears in `.sbl`; honor export filters in `Package.ll` when set.
- ABI stability: `@abi("symbol")` on exported members; semver + abi version in `.sbl` header.

## Agent rules

- Never tell users to compile dependency source from git checkouts for normal apps.
- Commit **`Package.lock`** for reproducible apps/CI (solution root or per-package).
- Path deps still compile together without packing; packed Std deps resolve via lock → store.

Docs: [docs/packages/package-manager.md](../../docs/packages/package-manager.md), [docs/toolchain/manifest.md](../../docs/toolchain/manifest.md), [docs/toolchain/solutions.md](../../docs/toolchain/solutions.md), [docs/formats/sbl.md](../../docs/formats/sbl.md).
