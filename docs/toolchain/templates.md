# Project templates

`lang new <template> -n <name>` scaffolds a directory with `Package.ll`, starter `.ll` sources, and skeleton files (`.gitignore`, `.editorconfig`, `README.md`, `.github/workflows/ci.yml`). Templates are **embedded in the `lang` binary** and expand `{{name}}`, `{{Name}}`, and `{{package}}`.

Phase 7 MVP uses Phase 6 builtins (`Args`, `File`, `Std.Llm` mocks) — not a full `Std.Cli` parser or HTTP listen socket (those land with Phase 8 stdlib).

Global options:

```bash
lang new <template> -n NAME [-o DIR | --path DIR] [--no-git]
lang new solution -n NAME [-o DIR]   # monorepo scaffold
```

`--no-git` skips `git init` (default is to initialize a repo when `git` is available).

---

## `console`

Minimal executable application.

```text
hello/
  Package.ll
  Sources/Hello/
    main.ll
  .gitignore
  README.md
```

`Package.ll`:

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "hello",
    products: [
        .executable(name: "hello", targets: ["Hello"]),
    ],
    targets: [
        .executableTarget(name: "Hello", path: "Sources/Hello"),
    ],
)
```

`Sources/Hello/main.ll`:

```ll
namespace Hello

import Std.IO

public func main() {
    print("Hello, LLLang!")
}
```

---

## `lib`

Publishable library with public API stub.

```text
acme-widgets/
  Package.ll
  Sources/Widgets/
    lib.ll
  Tests/WidgetsTests/
    lib_test.ll
  README.md
```

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
        .testTarget(name: "WidgetsTests", dependencies: ["Widgets"], path: "Tests/WidgetsTests"),
    ],
)
```

```ll
namespace Acme.Widgets

public struct Placeholder(message: String)
```

---

## `cli`

Command-line tool with argument parsing via `Args.count` / `Args.get` (full `Std.Cli` later).

```text
mycli/
  Package.ll
  Sources/MyCli/
    main.ll
    commands/
      run.ll
      version.ll
  Tests/MyCliTests/
    cli_test.ll
```

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "mycli",
    products: [
        .executable(name: "mycli", targets: ["MyCli"]),
    ],
    targets: [
        .executableTarget(name: "MyCli", path: "Sources/MyCli"),
        .testTarget(name: "MyCliTests", dependencies: ["MyCli"], path: "Tests/MyCliTests"),
    ],
)
```

```ll
namespace MyCli

import Std.Cli
import Std.IO

public func main(args: [String]) {
    let cmd = Cli.parse(args, commands: [RunCommand(), VersionCommand()])
    cmd.execute()
}
```

---

## `web`

Web-oriented layout: path router + `assets/` (no listen socket yet; Phase 8 adds `Std.Net`).

```text
myweb/
  Package.ll
  Sources/MyWeb/
    main.ll
    routes/
      health.ll
  assets/
    index.html
  Tests/MyWebTests/
    routes_test.ll
```

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "myweb",
    products: [
        .executable(name: "myweb", targets: ["MyWeb"]),
    ],
    targets: [
        .executableTarget(
            name: "MyWeb",
            path: "Sources/MyWeb",
            resources: [.copy("assets")],
        ),
        .testTarget(name: "MyWebTests", dependencies: ["MyWeb"], path: "Tests/MyWebTests"),
    ],
)
```

---

## `test`

Test-first package (library under test + harness).

```text
math-extra/
  Package.ll
  Sources/MathExtra/
    extra.ll
  Tests/MathExtraTests/
    extra_test.ll
  Tests/fixtures/
    cases.toon
```

Emphasizes `Tests/` and sample `@Test` functions.

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "math.extra",
    products: [
        .library(name: "MathExtra", targets: ["MathExtra"]),
    ],
    targets: [
        .target(name: "MathExtra", path: "Sources/MathExtra"),
        .testTarget(name: "MathExtraTests", dependencies: ["MathExtra"], path: "Tests/MathExtraTests"),
    ],
)
```

---

## `agent`

Automation / LLM agent project with `.mll` stub.

```text
my-agent/
  Package.ll
  Sources/MyAgent/
    main.ll
    agent.ll
  package.mll
  skills/
    README.md
```

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "my.agent",
    products: [
        .executable(name: "my-agent", targets: ["MyAgent"]),
    ],
    targets: [
        .executableTarget(name: "MyAgent", path: "Sources/MyAgent"),
    ],
)
```

```ll
namespace MyAgent

import Std.Llm

public func main() {
    let ctx = Llm.loadContext("package.mll")
    Llm.runAgent(ctx)
}
```

---

## `mcp`

[MCP](https://modelcontextprotocol.io/) server template.

```text
my-mcp/
  Package.ll
  Sources/MyMcp/
    main.ll
    server.ll
    tools/
      echo.ll
  mll/
    tools.mll
  README.md
```

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "my.mcp",
    products: [
        .executable(name: "my-mcp", targets: ["MyMcp"]),
    ],
    targets: [
        .executableTarget(
            name: "MyMcp",
            path: "Sources/MyMcp",
        ),
    ],
    settings: [
        .define("MCP_STDIO", .when(configuration: .debug)),
    ],
)
```

Documents stdio transport and tool registration patterns.

---

## `bench`

Benchmark harness.

```text
mybench/
  Package.ll
  Sources/MyBench/
    lib.ll
  benches/
    fib.ll
  Tests/MyBenchTests/
    smoke_test.ll
```

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "mybench",
    products: [
        .library(name: "MyBench", targets: ["MyBench"]),
    ],
    targets: [
        .target(name: "MyBench", path: "Sources/MyBench"),
        .testTarget(name: "MyBenchTests", dependencies: ["MyBench"], path: "Tests/MyBenchTests"),
    ],
)
```

```ll
// benches/fib.ll
@Bench
func fib20() {
    _ = fib(20)
}
```

Run with `lang bench` (Phase 4+).

---

## `solution`

Monorepo scaffold with root `Solution.ll`, shared libraries under `packages/`, and apps under `apps/`. See [Solutions](solutions.md) and `examples/shop-solution/`.

```text
shop/
  Solution.ll
  Package.lock              # after first lang restore (optional at create)
  packages/
    Core/
      Package.ll
      Sources/Core/
        geometry.ll
  apps/
    Cli/
      Package.ll
      Sources/Cli/
        main.ll
  README.md
```

`Solution.ll`:

```ll
// lang-tools-version: 0.1
import Std.SolutionDescription

let solution = Solution(
    name: "Shop",
    projects: [
        .package(path: "packages/Core"),
        .package(path: "apps/Cli"),
    ],
    defaultProjects: ["Cli"],
)
```

`packages/Core/Package.ll`:

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "shop.core",
    products: [
        .library(name: "Core", targets: ["Core"]),
    ],
    targets: [
        .target(name: "Core", path: "Sources/Core"),
    ],
)
```

`apps/Cli/Package.ll`:

```ll
// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "shop.cli",
    products: [
        .executable(name: "shop-cli", targets: ["Cli"]),
    ],
    dependencies: [
        .package(path: "../../packages/Core"),
    ],
    targets: [
        .executableTarget(
            name: "Cli",
            dependencies: [
                .product(name: "Core", package: "shop.core"),
            ],
            path: "Sources/Cli",
        ),
    ],
)
```

Create with:

```bash
lang new solution -n shop
```

Add a member later:

```bash
lang new lib -n Billing --path packages/Billing --solution .
lang solution add packages/Billing
```

---

## Template matrix

| Template | Products | Layout | Tests | `.mll` | Special dirs |
|----------|----------|--------|-------|--------|--------------|
| console | executable | `Sources/<Target>/` | — | — | — |
| lib | library | `Sources/` + `Tests/` | yes | — | — |
| cli | executable | `Sources/` | yes | — | `commands/` |
| web | executable | `Sources/` | yes | — | `assets/` |
| test | library | `Sources/` + `Tests/` | yes | — | `fixtures/` |
| agent | executable | `Sources/` | optional | yes | `skills/` |
| mcp | executable | `Sources/` | optional | yes | `mll/`, `tools/` |
| bench | library | `Sources/` + `Tests/` | smoke | — | `benches/` |
| solution | many packages | `Solution.ll` + `packages/` + `apps/` | per member | optional | — |

---

## Custom templates (planned)

`lang new --template-path ./my-template` will copy arbitrary trees with variable substitution (`{{name}}`, `{{package}}`).

---

## See also

- [CLI `new`](cli-reference.md)
- [Solutions](solutions.md)
- [Hello, world](../getting-started/hello-world.md)
