// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "cancellation",
    products: [.executable(name: "cancellation", targets: ["Cancellation"])],
    targets: [.executableTarget(name: "Cancellation", path: "src")],
)
