namespace McpEcho

import Std.IO
import Std.Llm

@tool(description: "Echo the input string")
public func echo(text: String) -> String {
    text
}

public func main() {
    var tools = ToolRegistry()
    tools.register(echo.self)
    print("mcp-echo ready")
}
