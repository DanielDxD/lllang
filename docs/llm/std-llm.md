# Std.Llm guide

**Std.Llm** is the standard library module for chat completion, streaming, tool dispatch, and structured output. Signatures live in [stdlib/llm.md](../stdlib/llm.md); this guide covers usage patterns and provider setup.

```ll
import Std.Llm
import Std.Json
```

## Minimal chat

```ll
public async func ask(model: ChatModel, question: String) async throws(LlmError) -> String {
    let req = ChatRequest(messages: [
        Message(role: .system, content: "You are a concise assistant."),
        Message(role: .user, content: question),
    ])
    let resp = try await model.complete(req)
    return resp.content
}
```

## Streaming

```ll
public async func streamAnswer(model: ChatModel, question: String) async throws(LlmError) {
    var req = ChatRequest(messages: [Message(role: .user, content: question)])
    for try await delta in model.stream(req) {
        print(delta.text, terminator: "")
    }
}
```

## Provider configuration

### Explicit

```ll
let model = OpenAIProvider(apiKey: Environment.get("OPENAI_API_KEY")!, model: "gpt-4.1")
```

### Environment convention

| Variable | Meaning |
|----------|---------|
| `LANG_LLM_PROVIDER` | `openai`, `anthropic`, `ollama`, … |
| `OPENAI_API_KEY` | OpenAI credential |
| `ANTHROPIC_API_KEY` | Anthropic credential |
| `OLLAMA_HOST` | Base URL for local Ollama |

```ll
public func defaultChatModel() throws(LlmError) -> ChatModel {
    switch Environment.getOrDefault("LANG_LLM_PROVIDER", default: "ollama") {
    case "openai": return OpenAIProvider(apiKey: require("OPENAI_API_KEY"))
    case "anthropic": return AnthropicProvider(apiKey: require("ANTHROPIC_API_KEY"))
    default: return OllamaProvider(model: "llama3")
    }
}
```

## System prompts with workspace context

```ll
public async func agentTurn(model: ChatModel, user: String) async throws(LlmError) -> String {
    let ctx = try await LlmContext.mergePackageContext("my.app")
    let req = ChatRequest(messages: [
        Message(role: .system, content: ctx),
        Message(role: .user, content: user),
    ])
    return try await model.complete(req).content
}
```

Generate `ctx` offline with [context generation](context-generation.md) or at startup from checked-in `.mll`.

## Tool calling loop

```ll
public async func runAgent(model: ChatModel, tools: ToolRegistry, user: String) async throws(LlmError) -> String {
    var messages = [Message(role: .user, content: user)]
    while true {
        let req = ChatRequest(messages: messages, tools: tools.schemas)
        let resp = try await model.complete(req)
        if let call = resp.toolCalls.first {
            let result = try await tools.dispatch(call)
            messages.append(Message(role: .assistant, content: resp.content))
            messages.append(Message(role: .tool, content: result, toolCallId: call.id))
            continue
        }
        return resp.content
    }
}
```

See [Tools & structured output](tools-and-structured-output.md) for `@tool` registration.

## Structured output

```ll
public struct TaskPlan: Codable {
    public var steps: [String]
    public var estimatedMinutes: Int
}

public async func plan(model: ChatModel, goal: String) async throws(LlmError) -> TaskPlan {
    try await model.structured(TaskPlan.self, prompt: "Plan: \(goal)")
}
```

Under the hood the client sets `responseFormat: .jsonSchema(JsonSchema.generate(TaskPlan.self))` and decodes with `JSON.decode` + strict limits.

## Error handling

| Error | Typical cause |
|-------|----------------|
| `rateLimited` | Back off using `retryAfter` |
| `invalidToolCall` | Model emitted malformed JSON arguments |
| `schemaMismatch` | Structured output failed validation |
| `transport` | Network / TLS |

Retry idempotent reads with exponential backoff; do not blindly retry mutating tools.

## Testing without providers

```ll
public struct StubModel: ChatModel {
    public var nextResponse: ChatResponse
    public func complete(_ request: ChatRequest) async throws(LlmError) -> ChatResponse { nextResponse }
    public func stream(_ request: ChatRequest) -> AsyncStream<ChatDelta> { AsyncStream { $0.finish() } }
}
```

## See also

- [Overview](overview.md)
- [MCP](mcp.md) — external hosts calling the same tools
