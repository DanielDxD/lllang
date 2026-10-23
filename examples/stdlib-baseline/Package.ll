// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "stdlib-baseline",
    products: [.executable(name: "stdlib-baseline", targets: ["StdlibBaseline"])],
    targets: [.executableTarget(name: "StdlibBaseline", path: "src")],
)
