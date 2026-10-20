// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "lib.geometry",
    products: [.library(name: "Geometry", targets: ["Geometry"])],
    targets: [.target(name: "Geometry", path: "src")],
)
