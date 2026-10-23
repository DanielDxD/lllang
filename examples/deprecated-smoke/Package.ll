// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "deprecated-smoke",
    products: [.executable(name: "deprecated-smoke", targets: ["DeprecatedSmoke"])],
    targets: [.executableTarget(name: "DeprecatedSmoke", path: "src")],
)
