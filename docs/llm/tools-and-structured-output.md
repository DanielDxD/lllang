# Tools and structured output

LLLang connects **compiler-known functions** to **LLM tool calls** via JSON Schema, and binds **model JSON** to **Codable** types for structured output.

Related: [Std.Llm](../stdlib/llm.md) · [Codable](../language/17-codable.md) · [Std.Json](../stdlib/json.md)

## `@tool` attribute

Mark functions exported to models:

```ll
import Std.Llm

@tool(description: "Search issues by label")
public async func searchIssues(label: String, limit: Int = 10) async throws(GitError) -> [IssueSummary] {
    try await git.search(label: label, max: limit)
}
```

The compiler emits:

- Stable **tool name** (default: function name; override with `@tool("search_issues")`)
- **Parameter schema** from types (`String`, `Int`, `UUID`, `Codable` structs, optional parameters → schema `required` rules)
- **Return schema** for documentation (providers may ignore return schema on the wire)

Collect schemas:

```ll
let schemas = [
    Tools.schema(for: searchIssues.self),
    Tools.schema(for: createIssue.self),
]
```

## JSON Schema generation

```ll
public enum JsonSchema {
    public static func generate(_ type: (some Any).Type) -> JsonSchemaDocument
    public static func generateParameters(_ function: (some Any).Type) -> JsonSchemaDocument
}
```

Structs used as tool parameters should use primary constructors and `Codable` field names:

```ll
public struct CreateIssueArgs(title: String, body: String, labels: [String] = []): Codable
```

### Optional and enum parameters

```ll
@tool(description: "Set priority")
public func setPriority(id: UUID, level: Priority?) throws(StoreError) { ... }

public enum Priority: String, Codable {
    case low, medium, high
}
```

Enums encode as string schemas with closed `enum` lists.

## Dispatching tool calls

```ll
public struct ToolCall: Codable {
    public var id: String
    public var name: String
    public var argumentsJson: String
}

public struct ToolRegistry {
    public var schemas: [ToolSchema] { get }
    public mutating func register(_ fn: (some Any).Type, handler: ToolHandler)
    public func dispatch(_ call: ToolCall) async throws(LlmError) -> String
}
```

Handlers decode `argumentsJson` with `JSON.decode` and `DecoderLimits`, invoke the LLLang function, then encode results:

```ll
public enum ToolResult {
    public static func encode<T: Encodable>(_ value: T) throws(EncodingError) -> String
    public static func text(_ message: String) -> String
}
```

## Structured output (non-tool)

When the model must return a typed document instead of calling functions:

```ll
public struct ReviewComment: Codable {
    public var line: Int
    public var severity: String
    public var message: String
}

let req = ChatRequest(
    messages: [Message(role: .user, content: diffText)],
    responseFormat: .jsonSchema(JsonSchema.generate(ReviewComment.self))
)
let resp = try await model.complete(req)
let comment: ReviewComment = try JSON.decode(resp.content.utf8Data, ReviewComment.self)
```

### Arrays and nesting

```ll
public struct Review: Codable {
    public var file: String
    public var comments: [ReviewComment]
}
```

Schema generation flattens associated types; large arrays should use streaming review patterns (multiple turns) to stay within output token limits.

## Validation strategy

1. **Schema first** — provider constrains syntax.
2. **Decode second** — `JSON.decode` with strict limits.
3. **Domain third** — validate invariants in application code (`expect`, `throws`).

```ll
public func parseReview(_ data: Data) throws(ReviewError) -> Review {
    let limits = DecoderLimits(maxDepth: 32, maxStringBytes: 1_000_000)
    let review: Review = try JSON.decode(data, Review.self, limits: limits)
    guard !review.comments.isEmpty else { throw ReviewError.empty }
    return review
}
```

## Agent pitfalls (document in `.mll`)

- Models invent enum cases — keep enums small and document unknown handling.
- Tool names must match registry exactly after renames — rerun `lang llm gen`.
- Do not expose destructive tools without confirmation layers in the host app.

## MCP overlap

External MCP hosts consume the same JSON Schema; LLLang MCP servers map MCP tool descriptors to `@tool` handlers ([MCP](mcp.md)).

## See also

- [Context generation](context-generation.md) — embed tool lists in `.mll` examples
- [Std.Toon](../stdlib/toon.md) — compact tool catalogs in TOON
