// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "errors",
    products: [.executable(name: "errors", targets: ["Errors"])],
    targets: [.executableTarget(name: "Errors", path: "src")],
)
