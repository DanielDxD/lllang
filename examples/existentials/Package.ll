// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "existentials",
    products: [.executable(name: "existentials", targets: ["Existentials"])],
    targets: [.executableTarget(name: "Existentials", path: "src")],
)
