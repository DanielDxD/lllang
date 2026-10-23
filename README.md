# LLLang

**LLLang** (`.ll`) is a statically typed, compiled, object-oriented programming language inspired by Swift and C# — improved where those languages leave room for better DX and performance.

- **Static & strict** — no null, typed errors, local type inference
- **No garbage collector** — ARC for classes, value semantics for structs/enums
- **Fast libraries** — dependencies ship as prebuilt `.bll` binaries + `.sbl` type declarations (no `node_modules`-style source trees)
- **LLM-native** — `.mll` metadata, `Std.Llm`, TOON/JSON first-class
- **Consistent toolkit** — `lang` CLI with .NET-style scaffolds

> **Status (v0.2 usable slice):** Phases 1–10 + 1.1–1.6 MVP — compile/run with LLVM, ARC, packages, templates, concurrency, sqlite, `HttpServer.serve`, installers. Freeze contract: [docs/stdlib/SHIPPED.md](docs/stdlib/SHIPPED.md). Still open: native TLS, Phase 1.7 (`tmp/1.2`). See [ROADMAP.md](ROADMAP.md).

## Install

**macOS / Linux**

```bash
curl -fsSL https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.sh | bash
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/lllang-lang/llang/main/scripts/install.ps1 | iex
```

Prebuilt binaries cover macOS (arm64/x64), Linux (x64/arm64), and Windows x64. End users need a host linker (`clang` / Xcode CLT) and sqlite — **not** LLVM 22. Full matrix and contributor build steps: [Installation](docs/getting-started/installation.md).

## Quick taste

```ll
namespace Shop.Models

import Std.Json
import Std.Toon

public struct Money(amount: Decimal, currency: String): Codable, Equatable

public enum OrderError: Error {
    invalidQuantity,
    outOfStock(sku: String),
}

public class Order(public let id: UUID): Codable {
    public private(set) var items: [Item] = []
    public weak var customer: Customer?

    public var total: Decimal => items.sum((i) => i.price.amount)

    public func add(item: Item) throws(OrderError) {
        guard item.quantity > 0 else { throw .invalidQuantity }
        items.append(item)
    }

    deinit { Log.debug("Order \(id) released") }
}

let order: Order = try JSON.decode(input)

let label = switch order.items.count {
    0 => "empty",
    1 => "single item",
    n where n > 100 => "bulk order",
    n => "\(n) items",
}

let shipping = computeShipping(order) catch (e) => Money(0, "USD")
print(TOON.encode(order))
```

## Philosophy

| Pillar | Meaning |
|--------|---------|
| Performance | Static dispatch by default, ARC + COW, LLVM backend, precompiled deps |
| DX | Primary constructors, expression `switch`/`if`, no `;`, no null, typed throws |
| LLM-ready | `.mll` context packs, TOON, structured tools in `Std.Llm` |
| Predictable | Local inference only, deterministic `deinit`, explicit `unsafe` |

See [docs/design/philosophy.md](docs/design/philosophy.md) and [docs/design/swift-vs-csharp-vs-lllang.md](docs/design/swift-vs-csharp-vs-lllang.md).

## File extensions

| Ext | Role |
|-----|------|
| `.ll` | Source program |
| `.bll` | Precompiled library binary (per target) |
| `.sbl` | Textual type declarations for a library |
| `.mll` | LLM-oriented metadata (TOON) |

## CLI (`lang`)

```bash
lang new console -n hello
lang build
lang run
lang add some.pkg@^1.2
lang test
lang lsp          # Language Server (stdio)
lang check src/   # Syntax check (phase 1)
lang llm gen      # Generate .mll
```

Each package is described by **`Package.ll`** (SwiftPM-style manifest DSL). Monorepos add root **`Solution.ll`** (.NET solution-style) — see `examples/shop-solution/` and [Solutions](docs/toolchain/solutions.md).

Templates: `console`, `lib`, `cli`, `web`, `test`, `agent`, `mcp`, `bench`, `solution`.

## Documentation

- [Documentation index](docs/index.md)
- [Getting started](docs/getting-started/installation.md)
- [Language guide](docs/language/01-lexical-structure.md)
- [Formal grammar](docs/spec/grammar.ebnf)
- [CLI reference](docs/toolchain/cli-reference.md)
- [Package manager](docs/packages/package-manager.md)
- [Standard library](docs/stdlib/overview.md)
- [LLM integration](docs/llm/overview.md)

## Building the toolchain (contributors)

Requires [Rust](https://rustup.rs/) 1.85+ and **LLVM 22** (`LLVM_SYS_221_PREFIX`). See [Installation](docs/getting-started/installation.md).

```bash
cargo build -p lang --release
cargo test
./target/release/lang --version
./target/release/lang check examples/hello
./scripts/pack-release.sh   # → dist/lang-<ver>-<triple>.tar.gz
```

VS Code / Cursor extension: see [editors/vscode](editors/vscode).  
Web highlighting (Shiki / highlight.js): see [editors/web](editors/web).

## Agent skills

Skills for AI coding agents live in [`skills/`](skills/) and are mirrored under [`.cursor/skills/`](.cursor/skills/). See [AGENTS.md](AGENTS.md).

## License

[MIT](LICENSE)
