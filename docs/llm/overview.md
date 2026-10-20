# LLM support in LLLang

LLLang treats **LLM-assisted development** as a first-class concern: compact metadata on disk (`.mll`), toolchain commands to assemble context, and **`Std.Llm`** for runtime chat, tools, and structured output in applications.

## Components

| Layer | Role |
|-------|------|
| [`.mll` format](../formats/mll.md) | TOON-encoded package metadata for agents |
| `lang llm` CLI | Generate metadata, bundle context, explain symbols |
| [Std.Toon](../stdlib/toon.md) | Encode/decode TOON; JSON round-trip |
| [Std.Llm](../stdlib/llm.md) | Chat models, `@tool`, structured output |
| [MCP](mcp.md) | Expose LLLang tools to external agent hosts |

## Workflows

### Authoring libraries for agents

1. Write public API in `.ll` with `///` docs.
2. Run `lang llm gen` to refresh machine sections of `.mll`.
3. Hand-edit `pitfalls`, `conventions`, and examples.
4. Check in `.mll` next to `Package.ll`.

### Building a coding agent in LLLang

1. Use `lang llm context` or `LlmContext.mergePackageContext` for system prompts.
2. Register `@tool` functions; schemas emit automatically.
3. Choose a [provider](std-llm.md) via config or constructor.
4. Parse structured replies with `Codable` + `JSON.decode`.

### MCP servers

Scaffold with `lang new mcp` and follow [MCP](mcp.md) to bridge host tools to your LLLang handlers.

## TOON vs JSON

- **TOON** default for `.mll` and agent context ([ADR 004](../design/adr/004-toon-for-mll.md)) — fewer tokens, line-oriented review.
- **JSON** for HTTP APIs and provider payloads — use `Std.Json` and `TOON.toJson` when converting.

## Security

- Never commit secrets in `.mll` or generated context bundles.
- Cap decode limits when ingesting model output (`DecoderLimits`).
- Validate tool arguments before invoking privileged code.

## Documentation map

- [Context generation](context-generation.md) — `lang llm gen`, `lang llm context`
- [`Std.Llm` guide](std-llm.md) — runtime API and providers
- [Tools & structured output](tools-and-structured-output.md) — `@tool`, JSON Schema, `Codable` binding
- [MCP](mcp.md) — Model Context Protocol integration

## See also

- [Standard library overview](../stdlib/overview.md)
- [CLI: `lang llm`](../toolchain/cli-reference.md)
