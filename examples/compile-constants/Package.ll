// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "compile-constants",
    products: [.executable(name: "compile-constants", targets: ["CompileConstants"])],
    targets: [.executableTarget(name: "CompileConstants", path: "src")],
)
