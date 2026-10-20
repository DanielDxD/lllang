// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "llm-agent",
    products: [.executable(name: "llm-agent", targets: ["LlmAgent"])],
    targets: [.executableTarget(name: "LlmAgent", path: "src")],
)
