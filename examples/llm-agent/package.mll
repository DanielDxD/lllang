version: "1"
package: llm-agent@0.1.0
summary: Minimal agent sample using Std.Llm ChatModel and @tool registrations.

concepts:
  - id: tools
    title: Structured tools
    body: Functions marked @tool expose typed schemas to the chat model via ToolRegistry.

api:
  LlmAgent:
    packageVersion:
      kind: func
      blurb: Returns package semver for sanity checks.
    echo:
      kind: func
      blurb: Echoes a note string for demos.

examples:
  - title: Run the agent
    lang: ll
    code: |
      import Std.Llm
      let model = ChatModel(provider: .openai, name: "gpt-4.1-mini")
      let agent = model.agent(context: Llm.loadContext("package.mll"), tools: ToolRegistry.current())

pitfalls:
  - Register @tool functions before building ToolRegistry.current().
  - Keep package.mll free of secrets; it ships with the repo.

conventions:
  - Tool names match function names in PascalCase modules.
  - Prefer TOON-friendly return types (structs with Codable).

tokens:
  priority: high
  prefer: [summary, api, pitfalls]
