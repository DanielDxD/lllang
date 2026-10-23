// lang-tools-version: 0.1
// Consumes `prebuilt-greet` via a path dependency (same layout as Solution path deps).
// For a true `.bll` link smoke: `cd ../prebuilt-greet && lang pack && lang publish --local`,
// then switch this package to `.package(id: "prebuilt-greet", from: "0.1.0")` + `lang restore`.
import Std.PackageDescription

let package = Package(
    name: "prebuilt-consumer",
    products: [.executable(name: "prebuilt-consumer", targets: ["Main"])],
    dependencies: [
        .package(path: "../prebuilt-greet"),
    ],
    targets: [
        .executableTarget(
            name: "Main",
            dependencies: [
                .product(name: "PrebuiltGreet", package: "prebuilt-greet"),
            ],
            path: "src",
        ),
    ],
)
