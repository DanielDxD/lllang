# Standard library overview

The LLLang standard library ships as prebuilt **`.bll`** artifacts referenced from your package manifest. Application code imports modules by name (`import Std.Json`); **`Std.Core`** is always in scope and does not require an import.

**Status:** API surface is stable in documentation; runtime implementations land in [Phase 3](../../ROADMAP.md). Signatures below are the contract importers and `.sbl` exports must match.

## Design principles

| Principle | Implication |
|-----------|-------------|
| Value-first | Structs and enums for data; classes for shared identity and I/O handles |
| Typed errors | `throws(SpecificError)` on fallible APIs; no bare `throws` in public stdlib |
| No blocking in async | Async modules suspend tasks; blocking calls live in sync modules or `runBlocking` bridges |
| Zero-copy where safe | `Span`, `Data`, JSON/TOON streaming, and COW collections avoid redundant copies |
| Agent-friendly | `Std.Toon`, `Std.Llm`, and `.mll` metadata share encoding conventions |

## Module index

| Module | Import | Focus |
|--------|--------|--------|
| [Core](core.md) | *(implicit)* | Builtins, `Result`, `Optional`, `Comparable`, runtime hooks |
| [Text](text.md) | `Std.Text` | `String`, encoding, parsing, formatting |
| [Collections](collections.md) | `Std.Collections` | `Array`, `Dictionary`, `Set`, COW buffers |
| [Math](math.md) | `Std.Math` | Numeric helpers, `Decimal`, vectors |
| [Time](time.md) | `Std.Time` | `Instant`, `Duration`, calendars, zones |
| [IO](io.md) | `Std.IO` | Readers, writers, buffering |
| [FS](fs.md) | `Std.FS` | Paths, files, directories, watching |
| [Process](process.md) | `Std.Process` | Subprocesses, signals, pipes |
| [Env](env.md) | `Std.Env` | Environment variables, args |
| [Net](net.md) | `Std.Net` | Sockets, addresses, TLS hooks |
| [Http](http.md) | `Std.Http` | Client, server, middleware |
| [Codable](codable.md) | `Std.Codable` | `Encoder`, `Decoder`, containers |
| [Json](json.md) | `Std.Json` | SIMD parse, streaming, zero-copy views |
| [Toon](toon.md) | `Std.Toon` | TOON encode/decode, tabular arrays |
| [Async](async.md) | `Std.Async` | `Task`, `TaskGroup`, `Channel` |
| [Sync](sync.md) | `Std.Sync` | Mutex, RWLock, semaphores |
| [Log](log.md) | `Std.Log` | Leveled logging, structured fields |
| [Testing](testing.md) | `Std.Testing` | `@Test`, expectations, fixtures |
| [Crypto](crypto.md) | `Std.Crypto` | Hashes, AEAD, secure random |
| [CLI](cli.md) | `Std.CLI` | Argument parsing, help, completions |
| [Llm](llm.md) | `Std.Llm` | Chat models, tools, structured output |
| [Memory](memory.md) | `Std.Memory` | `Arena`, `Pool`, scratch buffers |
| [FFI](ffi.md) | `Std.FFI` | C ABI helpers atop language `unsafe` |
| [PackageDescription](package-description.md) | `Std.PackageDescription` | Evaluate `Package.ll` manifests (toolchain only) |
| [SolutionDescription](solution-description.md) | `Std.SolutionDescription` | Evaluate `Solution.ll` monorepo roots (toolchain only) |

## Typical imports

```ll
import Std.Json
import Std.Http
import Std.Async

public async func main() throws(AppError) {
    let cfg = try JSON.decodeFile("config.json", Config.self)
    let client = HttpClient(base: cfg.apiBase)
    let body = try await client.get("/health")
    Log.info("health", fields: ["bytes": body.count])
}
```

## Platform and targets

Stdlib builds are published per target triple in the package registry. Your `Package.lock` pins exact `.bll` hashes. Features that need OS services (FS, Process, Net) are unavailable on bare-metal targets unless noted in module docs.

## Versioning

Stdlib semver follows the toolchain major version for breaking ABI changes. Patch releases are binary-compatible when `.sbl` signatures are unchanged. See [Versioning & ABI](../packages/versioning-and-abi.md).

## Related documentation

- Language: [Codable](../language/17-codable.md), [Concurrency](../language/12-concurrency.md), [Memory model](../language/11-memory-model.md), [FFI](../language/16-interop-ffi-unsafe.md)
- LLM: [Overview](../llm/overview.md), [`Std.Llm`](../llm/std-llm.md)
- Toolchain: [Manifest](../toolchain/manifest.md), [CLI](../toolchain/cli-reference.md)
