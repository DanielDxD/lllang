namespace {{Name}}

import Std.IO
import Std.Llm

public func main() async throws(LlmError) {
    let ctx = try await LlmContext.mergePackageContext("{{name}}")
    let model = OllamaProvider(model: "llama3")
    var tools = ToolRegistry()
    let answer = try await runAgent(model: model, tools: tools, user: "hello")
    print(answer)
}
