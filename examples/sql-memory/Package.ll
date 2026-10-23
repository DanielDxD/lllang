// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "sql-memory",
    products: [.executable(name: "sql-memory", targets: ["SqlMemory"])],
    targets: [.executableTarget(name: "SqlMemory", path: "src")],
)
