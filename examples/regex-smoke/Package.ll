// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "regex-smoke",
    products: [.executable(name: "regex-smoke", targets: ["RegexSmoke"])],
    targets: [.executableTarget(name: "RegexSmoke", path: "src")],
)
