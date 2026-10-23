// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "parallel-run",
    products: [.executable(name: "parallel-run", targets: ["ParallelRun"])],
    targets: [.executableTarget(name: "ParallelRun", path: "src")],
)
