// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "protocol-constraints",
    products: [.executable(name: "protocol-constraints", targets: ["ProtocolConstraints"])],
    targets: [.executableTarget(name: "ProtocolConstraints", path: "src")],
)
