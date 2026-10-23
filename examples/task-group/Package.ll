// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "task-group",
    products: [.executable(name: "task-group", targets: ["TaskGroupEx"])],
    targets: [.executableTarget(name: "TaskGroupEx", path: "src")],
)
