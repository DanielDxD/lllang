// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "{{name}}.cli",
    products: [.executable(name: "cli", targets: ["Cli"])],
    dependencies: [.package(path: "../../packages/Core")],
    targets: [.executableTarget(name: "Cli", path: "Sources/Cli", dependencies: ["Core"])],
)
