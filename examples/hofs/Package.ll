// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "hofs",
    products: [.executable(name: "hofs", targets: ["Main"])],
    targets: [.executableTarget(name: "Main", path: "src")],
)
