// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "actors-cancel",
    products: [.executable(name: "actors-cancel", targets: ["ActorsCancel"])],
    targets: [.executableTarget(name: "ActorsCancel", path: "src")],
)
