namespace {{Name}}

import Std.IO
import Std.Llm

@tool(description: "Echo the input string")
public func echo(text: String) -> String {
    text
}

public func main() async throws(LlmError) {
    var tools = ToolRegistry()
    tools.register(echo.self)
    print("mcp server scaffold — register tools and serve over stdio")
}
