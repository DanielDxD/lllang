// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "json-toon",
    products: [.executable(name: "json-toon", targets: ["JsonToon"])],
    targets: [.executableTarget(name: "JsonToon", path: "src")],
)
