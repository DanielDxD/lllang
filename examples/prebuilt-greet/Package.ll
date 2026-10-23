// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "prebuilt-greet",
    products: [.library(name: "PrebuiltGreet", targets: ["PrebuiltGreet"])],
    targets: [.target(name: "PrebuiltGreet", path: "src")],
)
