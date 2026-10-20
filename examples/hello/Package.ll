// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "hello",
    products: [.executable(name: "hello", targets: ["Hello"])],
    targets: [.executableTarget(name: "Hello", path: "src")],
)
