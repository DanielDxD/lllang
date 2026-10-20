# Project anatomy

An LLLang **package** is a directory with a `Package.ll` manifest (SwiftPM-style), sources under `Sources/`, optional tests under `Tests/`, and one or more **products**. A **solution** (`Solution.ll`) groups many packages for monorepos (.NET-style).

## Top-level layout (single package)

```text
MyApp/
  Package.ll               # required manifest (LLLang DSL)
  Package.lock             # generated lockfile (after restore/add)
  .lang/                   # local cache (gitignored); build artifacts
  Sources/
    MyApp/                 # default: Sources/<TargetName>/
      main.ll
  Tests/
    MyAppTests/
      MyAppTests.ll
  docs/                    # optional human docs
  assets/                  # bundled resources (referenced from Package.ll)
```

## `Package.ll`

Declarative LLLang, not TOML. Full reference: [Manifest](../toolchain/manifest.md).

Minimal executable:

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "my-app",
    products: [
        .executable(name: "my-app", targets: ["MyApp"]),
    ],
    targets: [
        .executableTarget(name: "MyApp", path: "Sources/MyApp"),
    ],
)
```

Minimal library:

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "acme.widgets",
    products: [
        .library(name: "Widgets", targets: ["Widgets"]),
    ],
    targets: [
        .target(name: "Widgets", path: "Sources/Widgets"),
        .testTarget(name: "WidgetsTests", dependencies: ["Widgets"]),
    ],
)
```

## Source tree

| Pattern | Role |
|---------|------|
| `Sources/<Target>/main.ll` | Entry for executable targets |
| `Sources/<Target>/**/*.ll` | Module sources |
| `Tests/<Target>/**/*.ll` | `lang test` discovery |

**Namespaces** in source (`namespace Acme.Widgets`) align with package name segments by convention; the compiler resolves modules via `import` and package targets.

## Lockfile (`Package.lock`)

After `lang add`, `lang update`, or `lang restore`, the lockfile records exact versions and content hashes of `.bll` / `.sbl` blobs. Commit it for applications. Never hand-edit.

## Dependencies on disk

There is **no** `node_modules` tree of source. Resolved packages live in the global store:

```text
~/.lang/store/<hash>/...
  Package.ll          # cached manifest snapshot
  package.sbl
  targets/
    aarch64-apple-darwin.bll
```

See [Package manager](../packages/package-manager.md).

## Solution layout (monorepo)

```text
Shop/
  Solution.ll
  Package.lock              # shared lock (optional)
  packages/
    Core/Package.ll
    Billing/Package.ll
  apps/
    Api/Package.ll
    Cli/Package.ll
```

```ll
// Solution.ll
// lang-tools-version: 0.1
import Std.SolutionDescription

let solution = Solution(
    name: "Shop",
    projects: [
        .package(path: "packages/Core"),
        .package(path: "packages/Billing"),
        .package(path: "apps/Api"),
        .package(path: "apps/Cli"),
    ],
    defaultProjects: ["Api", "Cli"],
)
```

Details: [Solutions](../toolchain/solutions.md).

## Artifacts you ship

| Kind | Ship to registry | Ship to end users |
|------|------------------|-------------------|
| Executable product | — | Native binary |
| Library product | `.sbl` + `.bll` per target | Via dependency |
| Agent / MCP package | Package + `.mll` | Template + metadata |

## `.gitignore` recommendations

```gitignore
.lang/
*.bll
Package.lock   # only if you intentionally float deps (unusual)
```

## Related docs

- [Manifest](../toolchain/manifest.md)
- [Solutions](../toolchain/solutions.md)
- [Templates](../toolchain/templates.md)
- [`.ll` format](../formats/ll.md)
