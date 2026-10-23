// lang-tools-version: 0.1
import Std.PackageDescription

let package = Package(
    name: "inheritance",
    products: [.executable(name: "inheritance", targets: ["Inheritance"])],
    targets: [.executableTarget(name: "Inheritance", path: "src")],
)
