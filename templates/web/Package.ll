// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "{{name}}",
    products: [.executable(name: "{{name}}", targets: ["{{Name}}"])],
    targets: [.executableTarget(name: "{{Name}}", path: "Sources/{{Name}}")],
)
