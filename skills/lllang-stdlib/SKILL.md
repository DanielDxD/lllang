---
name: lllang-stdlib
description: Use LLLang standard library modules (Std.Json, Std.Http, Std.IO, Std.Async, etc.). Use when calling documented std APIs or drafting imports in examples.
---

# Standard library

## Basics

- **`Std.Core`** is implicit (Optional, Result, Error, Equatable, etc.).
- Import by module: `import Std.Json`, `import Std.Http`, `import Std.IO`.
- Stdlib ships as prebuilt `.bll` + `.sbl`; pin via `lang.lock`.

## Common modules

| Module | Use |
|--------|-----|
| `Std.IO` | `print`, readers/writers |
| `Std.Json` / `Std.Toon` | Serialization |
| `Std.Http` | `Http.get`, clients, server (Phase 3 runtime) |
| `Std.Async` | Tasks, groups, channels |
| `Std.Llm` | ChatModel, agents, tools |
| `Std.Log` | Structured logging |
| `Std.Testing` | `@Test`, expectations |

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

Full index: [docs/stdlib/overview.md](../../docs/stdlib/overview.md). API contracts may be *planned* until Phase 3 runtime — match documented signatures in examples.
