// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "{{name}}",
    products: [
        .library(name: "{{Name}}", targets: ["{{Name}}"]),
    ],
    targets: [
        .target(name: "{{Name}}", path: "Sources/{{Name}}"),
        .testTarget(name: "{{Name}}Tests", dependencies: ["{{Name}}"], path: "Tests/{{Name}}Tests"),
    ],
)
