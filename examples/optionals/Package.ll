// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "optionals",
    products: [.executable(name: "optionals", targets: ["Optionals"])],
    targets: [.executableTarget(name: "Optionals", path: "src")],
)
