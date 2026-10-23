// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "identity",
    products: [.executable(name: "identity", targets: ["IdentityExample"])],
    targets: [.executableTarget(name: "IdentityExample", path: "src")],
)
