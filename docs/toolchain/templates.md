# Project templates

`lang new <template> -n <name>` scaffolds a directory with `lang.toml`, starter `.ll` sources, and optional CI/editor files. Templates align with [manifest](manifest.md) conventions and [project anatomy](../getting-started/project-anatomy.md).

Global options:

```bash
lang new <template> -n NAME [--path DIR] [--no-git]
```

---

## `console`

Minimal executable application.

```text
hello/
  lang.toml
  src/
    main.ll
  .gitignore
  README.md
```

`lang.toml`:

```toml
[package]
name = "hello"
version = "0.1.0"
kind = "app"
entry = "src/main.ll"
```

`src/main.ll`:

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
  lang.toml
  src/
    lib.ll
  tests/
    lib_test.ll
  README.md
```

```toml
[package]
name = "acme.widgets"
version = "0.1.0"
kind = "lib"

[build]
public-api = ["src/**/*.ll"]
```

```ll
namespace Acme.Widgets

public struct Placeholder(message: String)
```

---

## `cli`

Command-line tool with argument parsing (uses `Std.Cli` when available).

```text
mycli/
  lang.toml
  src/
    main.ll
    commands/
      run.ll
      version.ll
  tests/
    cli_test.ll
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

Web-oriented layout (HTTP server + static assets).

```text
myweb/
  lang.toml
  src/
    main.ll
    routes/
      health.ll
  assets/
    index.html
  tests/
    routes_test.ll
```

```toml
[package.resources]
paths = ["assets/**"]
```

---

## `test`

Test-first package (library under test + harness).

```text
math-extra/
  lang.toml
  src/
    extra.ll
  tests/
    extra_test.ll
  tests/fixtures/
    cases.toon
```

Emphasizes `tests/` and sample `@Test` functions.

---

## `agent`

Automation / LLM agent project with `.mll` stub.

```text
my-agent/
  lang.toml
  src/
    main.ll
    agent.ll
  package.mll
  skills/
    README.md
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
  lang.toml
  src/
    main.ll
    server.ll
    tools/
      echo.ll
  mll/
    tools.mll
  README.md
```

```toml
[package]
name = "my.mcp"
kind = "app"
entry = "src/main.ll"

[build]
define = { MCP_STDIO = "true" }
```

Documents stdio transport and tool registration patterns.

---

## `bench`

Benchmark harness.

```text
mybench/
  lang.toml
  src/
    lib.ll
  benches/
    fib.ll
  tests/
    smoke_test.ll
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

## Template matrix

| Template | `kind` | Entry | Tests | `.mll` | Special dirs |
|----------|--------|-------|-------|--------|--------------|
| console | app | `main.ll` | — | — | — |
| lib | lib | — | yes | — | — |
| cli | app | `main.ll` | yes | — | `commands/` |
| web | app | `main.ll` | yes | — | `assets/` |
| test | lib | — | yes | — | `fixtures/` |
| agent | app | `main.ll` | optional | yes | `skills/` |
| mcp | app | `main.ll` | optional | yes | `mll/`, `tools/` |
| bench | lib | — | smoke | — | `benches/` |

---

## Custom templates (planned)

`lang new --template-path ./my-template` will copy arbitrary trees with variable substitution (`{{name}}`, `{{package}}`).

---

## See also

- [CLI `new`](cli-reference.md)
- [Hello, world](../getting-started/hello-world.md)
