// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "shapes-oop",
    products: [.executable(name: "shapes-oop", targets: ["ShapesOop"])],
    targets: [.executableTarget(name: "ShapesOop", path: "src")],
)
