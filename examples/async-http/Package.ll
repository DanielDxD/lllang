// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "async-http",
    products: [.executable(name: "async-http", targets: ["AsyncHttp"])],
    targets: [.executableTarget(name: "AsyncHttp", path: "src")],
)
