// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "nested-codable",
    products: [.executable(name: "nested-codable", targets: ["NestedCodable"])],
    targets: [.executableTarget(name: "NestedCodable", path: "src")],
)
