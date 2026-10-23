// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "{{package}}.core",
    products: [.library(name: "Core", targets: ["Core"])],
    targets: [.target(name: "Core", path: "Sources/Core")],
)
