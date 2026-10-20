namespace LlmAgent

import Std.IO
import Std.Llm

public async func main() throws(LlmError) {
    let ctx = try await LlmContext.mergePackageContext("llm-agent")
    let model: ChatModel = OllamaProvider(model: "llama3")
    var tools = ToolRegistry()
    tools.register(packageVersion.self)
    tools.register(echo.self)
    let answer = try await runAgent(
        model: model,
        tools: tools,
        user: "Summarize this package",
    )
    print(answer)
}
