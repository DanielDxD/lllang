// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "identity-smoke",
    products: [.executable(name: "identity-smoke", targets: ["IdentitySmoke"])],
    targets: [.executableTarget(name: "IdentitySmoke", path: "src")],
)
