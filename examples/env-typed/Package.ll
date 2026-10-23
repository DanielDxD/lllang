// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "env-typed",
    products: [.executable(name: "env-typed", targets: ["EnvTyped"])],
    targets: [.executableTarget(name: "EnvTyped", path: "src")],
)
