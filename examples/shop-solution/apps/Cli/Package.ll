// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "shop.cli",
    products: [.executable(name: "shop-cli", targets: ["Cli"])],
    dependencies: [
        .package(path: "../../packages/Core"),
    ],
    targets: [
        .executableTarget(
            name: "Cli",
            dependencies: [
                .product(name: "Core", package: "shop.core"),
            ],
            path: "src",
        ),
    ],
)
