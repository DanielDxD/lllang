// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "interop-c",
    products: [.executable(name: "interop-c", targets: ["Main"])],
    targets: [.executableTarget(name: "Main", path: "src")],
)
