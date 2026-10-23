// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "extensions",
    products: [.executable(name: "extensions", targets: ["Extensions"])],
    targets: [.executableTarget(name: "Extensions", path: "src")],
)
