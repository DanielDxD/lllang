// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "mcp-echo",
    products: [.executable(name: "mcp-echo", targets: ["McpEcho"])],
    targets: [.executableTarget(name: "McpEcho", path: "src")],
)
