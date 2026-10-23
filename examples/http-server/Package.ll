// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "http-server",
    products: [.executable(name: "http-server", targets: ["Main"])],
    targets: [.executableTarget(name: "Main", path: "src")],
)
