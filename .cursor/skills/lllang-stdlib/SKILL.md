---
name: lllang-stdlib
description: Use LLLang standard library modules (Std.Json, Std.Http, Std.IO, Std.Async, etc.). Use when calling documented std APIs or drafting imports in examples.
---

# Standard library

## Basics

- **`Std.Core`** is implicit (Optional, Result, Error, Equatable, etc.).
- Import by module: `import Std.Json`, `import Std.Http`, `import Std.IO`.
- Phase 3 MVP: Json/Toon/Http/Async/Llm run via runtime helpers + prelude; fuller surfaces stay documented contracts.

## Common modules

| Module | Use |
|--------|-----|
| `Std.IO` | `print`, readers/writers |
| `Std.Json` / `Std.Toon` | `JSON.encode` / `decode`, `TOON.encode` / `decode` (MVP) |
| `Std.Http` | `Http.get(url) async throws(HttpError) -> String` (MVP) |
| `Std.Async` | `async`/`await` cooperative MVP |
| `Std.Llm` | `runAgent`, tools, `OllamaProvider`, mock via `LANG_LLM_MOCK` |
| `Std.Log` | Structured logging (documented) |
| `Std.Testing` | `@suite` / `@test` / `@theory` / `@values`; macros `@expect`, `@equal`, `@empty`/`@notEmpty`, `@gte`/`@lte`, …; `lang test` |

## Async HTTP pattern

```ll
import Std.Http
import Std.Json

public async func fetch() throws(HttpError) -> Model {
    let body = try await Http.get("https://example.com/data")
    try JSON.decode(body)
}
```

## Design constraints (public API)

- Typed `throws(SpecificError)` — avoid bare `throws` in library surface.
- Blocking I/O stays in sync modules; async modules suspend tasks.
- Value types for data; classes for shared identity and handles.

Full index: [docs/stdlib/overview.md](../../docs/stdlib/overview.md). Match signatures in `examples/json-toon`, `async-http`, and `llm-agent`.
