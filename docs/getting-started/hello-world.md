# Hello, world

This walkthrough creates a minimal console application, explains what Phase 1 can do today, and shows how the same project will build and run once codegen lands.

## Create a project

Scaffold from the **console** template (Phase 2+; until then, create files manually or copy from `examples/hello` when present):

```bash
lang new console -n hello
cd hello
```

A generated console app looks like this:

```ll
namespace Hello

import Std.IO

public func main() {
    print("Hello, LLLang!")
}
```

## Project layout

Every app and library uses the same skeleton (details in [Project anatomy](project-anatomy.md)):

```text
hello/
  lang.toml          # package manifest
  lang.lock          # pinned dependency graph (after lang add)
  src/
    main.ll          # entry for executables
  tests/
    hello_test.ll    # optional; lang test
```

## Phase 1: syntax check

With Phase 1 tooling, compile and run are not available yet. Validate syntax:

```bash
lang check .
lang check src/main.ll
```

`check` walks all `.ll` files under the path, parses them, and prints diagnostics with file, line, and column. Exit code is non-zero if any file fails to parse.

## Phase 2+: build and run

When the LLVM backend is enabled:

```bash
lang build              # debug, native target
lang build --release
lang run                  # build if needed, then execute
lang run -- arg1 arg2
```

`lang run` respects `[build]` settings in `lang.toml` (output name, target triple, optimization).

## Watch mode (planned)

During development:

```bash
lang watch run
```

Rebuilds incrementally when `.ll` files change and re-runs the executable. Watch integrates with the same build graph as `lang build`.

## Add a dependency (planned)

Libraries publish as `.sbl` + `.bll` pairs — consumers never recompile dependency source:

```bash
lang add std.io@^0.1
lang restore
```

Resolution prefers `.sbl` for the type checker and loads matching `.bll` artifacts from the global store. See [Package manager](../packages/package-manager.md).

## Documentation comments

Public APIs in libraries should use `///` docs; the LSP shows them on hover (Phase 1: syntax-only hover where implemented):

```ll
/// Greets the user with a fixed message.
public func greet(name: String) {
    print("Hello, \(name)!")
}
```

Generate HTML docs later with `lang doc` (planned).

## Common mistakes

| Mistake | Fix |
|---------|-----|
| Using `null` outside `T?` | LLLang has no null; use optionals or `Result` |
| Swift-style labels `func f(label x: Int)` | Use `func f(x: Int)` — no external argument labels |
| `new Type()` | Use `Type(...)` or literals; no `new` keyword |
| Semicolon after every line | Optional; newlines usually suffice |

## Next steps

- [Project anatomy](project-anatomy.md)
- [`.ll` source format](../formats/ll.md)
- [CLI reference](../toolchain/cli-reference.md)
- Language guide starting at [Lexical structure](../language/01-lexical-structure.md)
