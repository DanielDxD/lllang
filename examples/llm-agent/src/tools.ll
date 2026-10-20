namespace LlmAgent

import Std.Llm

@tool(description: "Return the semver from lang.toml for this package.")
public func packageVersion() -> String {
    "0.1.0"
}

@tool(description: "Echo a short note back to the model.")
public func echo(note: String) -> String {
    note
}
