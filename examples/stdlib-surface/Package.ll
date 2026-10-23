// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "stdlib-surface",
    products: [.executable(name: "stdlib-surface", targets: ["StdlibSurface"])],
    targets: [.executableTarget(name: "StdlibSurface", path: "src")],
)
