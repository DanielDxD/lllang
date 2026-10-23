// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "weak-cycle",
    products: [.executable(name: "weak-cycle", targets: ["WeakCycle"])],
    targets: [.executableTarget(name: "WeakCycle", path: "src")],
)
