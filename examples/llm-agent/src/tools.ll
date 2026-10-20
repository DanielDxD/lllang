namespace LlmAgent

import Std.Llm

@tool(description: "Return the package version from Package.ll / lock metadata.")
public func packageVersion() -> String {
    "0.1.0"
}

@tool(description: "Echo a short note back to the model.")
public func echo(note: String) -> String {
    note
}
