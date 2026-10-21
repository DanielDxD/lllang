// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "unit-tests",
    products: [.library(name: "UnitTests", targets: ["UnitTests"])],
    targets: [
        .target(name: "UnitTests", path: "Sources/UnitTests"),
        .testTarget(name: "UnitTestsTests", dependencies: ["UnitTests"], path: "Tests/UnitTestsTests"),
    ],
)
