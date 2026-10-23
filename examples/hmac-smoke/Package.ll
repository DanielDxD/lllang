// lang-tools-version: 0.1
import Std.PackageDescription
let package = Package(
    name: "hmac-smoke",
    products: [.executable(name: "hmac-smoke", targets: ["Main"])],
    targets: [.executableTarget(name: "Main", path: "src")],
)
