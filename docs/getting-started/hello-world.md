# Hello, world

This walkthrough creates a minimal console application and runs it with the Phase 2 toolchain (`lang check` / `lang build` / `lang run`).

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
  Package.ll         # package manifest (SwiftPM-style DSL)
  Package.lock       # pinned dependency graph (after lang add)
  Sources/Hello/
    main.ll          # entry for executables
  Tests/HelloTests/
    hello_test.ll    # optional; lang test
```

## Check

```bash
lang check .
lang check src/main.ll
```

`check` parses and runs semantic analysis (LL1xxx / LL2xxx). Exit code is non-zero on errors.

## Build and run

Requires LLVM 22 (`LLVM_SYS_221_PREFIX`). From a package directory or via `--manifest`:

```bash
lang build --manifest Package.ll
lang build --manifest Package.ll --release
lang run --manifest Package.ll
lang run --manifest Package.ll -- -- arg1 arg2
```

Copy `examples/hello` for a known-good layout:

```bash
lang run --manifest examples/hello/Package.ll
# → Hello, LLLang!
```

`lang run` respects build settings from `Package.ll` (`settings:`, product name, `--release`).

## Watch mode

During development:

```bash
lang watch
```

Rebuilds incrementally when `.ll` files change (MVP). See [CLI reference](../toolchain/cli-reference.md).

## Add a dependency

Libraries publish as `.sbl` + `.bll` pairs — consumers never recompile dependency source:

```bash
lang add acme.math@0.1.0
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
