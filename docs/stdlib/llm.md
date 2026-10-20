# Std.Llm

**Std.Llm** integrates chat models, tool calling, structured output, and provider configuration for agent-style applications.

```ll
import Std.Llm
```

Deep dives: [LLM overview](../llm/overview.md) · [Tools & structured output](../llm/tools-and-structured-output.md) · [`Std.Llm` guide](../llm/std-llm.md).

## Messages

```ll
public enum Role {
    case system, user, assistant, tool
}

public struct Message {
    public var role: Role
    public var content: String
    public var name: String?
    public var toolCallId: String?
    public init(role: Role, content: String)
}
```

## Chat model protocol

```ll
public protocol ChatModel: Sendable {
    func complete(_ request: ChatRequest) async throws(LlmError) -> ChatResponse
    func stream(_ request: ChatRequest) -> AsyncStream<ChatDelta>
}

public struct ChatRequest {
    public var messages: [Message]
    public var tools: [ToolSchema] = []
    public var temperature: Float64 = 0.7
    public var maxTokens: Int?
    public var responseFormat: ResponseFormat = .text
}

public enum ResponseFormat {
    case text
    case jsonSchema(JsonSchemaDocument)
}
```

## Tool definitions

Functions annotated with `@tool` export JSON Schema for providers:

```ll
@tool(description: "Look up a user by id")
public func lookupUser(id: UUID) async throws(ApiError) -> UserSummary {
    try await api.fetchUser(id)
}

public struct ToolSchema: Codable {
    public var name: String
    public var description: String
    public var parameters: JsonSchemaDocument
}

public enum Tools {
    public static func schema(for function: (some Any).Type) -> ToolSchema
    public static func register(_ schemas: [ToolSchema], handler: ToolDispatcher)
}
```

Runtime dispatches model-emitted tool calls back to registered LLLang functions with typed decoding of arguments.

## Structured output

Bind model JSON to `Codable` types:

```ll
public struct ExtractionResult: Codable {
    public var title: String
    public var tags: [String]
}

public extension ChatModel {
    public func structured<T: Codable>(_ type: T.Type, prompt: String) async throws(LlmError) -> T {
        let req = ChatRequest(
            messages: [Message(role: .user, content: prompt)],
            responseFormat: .jsonSchema(JsonSchema.generate(T.self))
        )
        let resp = try await complete(req)
        try JSON.decode(resp.content.data(using: .utf8)!, T.self)
    }
}
```

## Providers

```ll
public struct OpenAIProvider: ChatModel {
    public init(apiKey: String, model: String = "gpt-4.1")
}

public struct AnthropicProvider: ChatModel {
    public init(apiKey: String, model: String = "claude-sonnet-4")
}

public struct OllamaProvider: ChatModel {
    public init(base: URL = URL("http://127.0.0.1:11434"), model: String)
}
```

Configure via environment (`LANG_LLM_PROVIDER`, provider-specific keys) or explicit constructors.

## Context from `.mll`

```ll
public enum LlmContext {
    public static func loadMll(_ path: Path) throws(LlmError) -> String
    public static func mergePackageContext(_ package: String) async throws(LlmError) -> String
}
```

Uses TOON from [Std.Toon](toon.md) and workspace metadata from `lang llm context`.

## Errors

```ll
public enum LlmError: Error {
    case transport(String)
    case rateLimited(retryAfter: Duration?)
    case invalidToolCall(String)
    case schemaMismatch(String)
    case providerStatus(Int, String)
}
```

## See also

- [Toon](toon.md) — compact context on the wire
- [MCP](../llm/mcp.md) — expose tools to external agents
