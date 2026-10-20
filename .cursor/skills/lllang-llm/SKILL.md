---
name: lllang-llm
description: Build LLM agents with Std.Llm, ChatModel, @tool functions, and package.mll TOON context. Use for agent templates, MCP-style tools, and lang llm workflows.
---

# LLLang LLM integration

## Artifacts

| File | Role |
|------|------|
| `package.mll` | TOON metadata: summary, api index, pitfalls, examples |
| `@tool` on functions | Exposes typed tools to the model via schema generation |
| `@mll(...)` on types | Hints for `lang llm gen` (optional) |

`.mll` is for agents; **types** still come from `.ll` / `.sbl`.

## Typical agent entry

```ll
import Std.Llm

@tool(description: "Short human-readable purpose")
public func lookup(id: String) async throws(ServiceError) -> Item { ... }

public async func main() async throws(LlmError) {
    let ctx = try await LlmContext.mergePackageContext("my.app")
    let model: ChatModel = OllamaProvider(model: "llama3")
    var tools = ToolRegistry()
    tools.register(lookup.self)
    let answer = try await runAgent(model: model, tools: tools, user: "prompt")
}
```

- `ChatModel` is a protocol; construct `OpenAIProvider`, `AnthropicProvider`, or `OllamaProvider`.
- Register each `@tool` function on `ToolRegistry` before `runAgent`.
- Prefer `Codable` return types for structured tool results.
- Keep secrets out of `.mll`; use env/config via `Std.Env`.

## CLI (planned)

```bash
lang llm gen [-p PACKAGE]
lang llm context [--max-tokens N]
lang llm explain Module.Type
```

Example project: [examples/llm-agent](../../examples/llm-agent). Schema reference: [docs/formats/mll.md](../../docs/formats/mll.md).
