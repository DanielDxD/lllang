// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "stdlib-ffi",
    products: [.executable(name: "stdlib-ffi", targets: ["StdlibFfi"])],
    targets: [.executableTarget(name: "StdlibFfi", path: "src")],
)
