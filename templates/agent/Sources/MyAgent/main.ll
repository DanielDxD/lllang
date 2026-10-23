namespace {{Name}}

import Std.IO
import Std.Llm

@tool(description: "Return the package name.")
public func packageName() -> String {
    "{{name}}"
}

@tool(description: "Echo a short note.")
public func echo(note: String) -> String {
    note
}

public func main() throws(LlmError) {
    let _ctx = try LlmContext.mergePackageContext("{{name}}")
    let model: ChatModel = OllamaProvider(model: "llama3")
    var tools = ToolRegistry()
    tools.register(packageName.self)
    tools.register(echo.self)
    let answer = try runAgent(
        model: model,
        tools: tools,
        user: "Say hello using the tools",
    )
    print(answer)
}
