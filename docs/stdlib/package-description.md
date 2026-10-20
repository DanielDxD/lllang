# `Std.PackageDescription`

API surface evaluated when loading **`Package.ll`**. Not linked into application binaries.

```ll
namespace Std.PackageDescription

public struct Package(
    name: String,
    platforms: [Platform] = [],
    products: [Product],
    dependencies: [PackageDependency] = [],
    targets: [Target],
    defaultLocalization: String? = nil,
    settings: [BuildSetting] = [],
)

public enum Platform {
    macOS(String),
    iOS(String),
    linux,
    windows,
    native,
}

public enum Product {
    executable(name: String, targets: [String]),
    library(name: String, targets: [String], type: LibraryType = .automatic),
}

public enum LibraryType {
    automatic,
    static,
    dynamic,
}

public enum PackageDependency {
    registry(id: String, requirement: VersionRequirement),
    url(url: String, requirement: VersionRequirement),
    path(path: String),
}

public enum VersionRequirement {
    from(String),
    exact(String),
    range(String),
    branch(String),
    revision(String),
}

// Factories used in Package.ll (sugar over the enums above)
extension PackageDependency {
    public static func package(id: String, from: String) -> PackageDependency
    public static func package(id: String, exact: String) -> PackageDependency
    public static func package(id: String, range: String) -> PackageDependency
    public static func package(url: String, from: String) -> PackageDependency
    public static func package(path: String) -> PackageDependency
}

public enum Target {
    target(
        name: String,
        dependencies: [TargetDependency] = [],
        path: String? = nil,
        exclude: [String] = [],
        resources: [Resource] = [],
        settings: [BuildSetting] = [],
    ),
    executableTarget(
        name: String,
        dependencies: [TargetDependency] = [],
        path: String? = nil,
        settings: [BuildSetting] = [],
    ),
    testTarget(
        name: String,
        dependencies: [TargetDependency] = [],
        path: String? = nil,
    ),
    binaryTarget(name: String, path: String),
    binaryTarget(name: String, url: String, checksum: String),
}

public enum TargetDependency {
    byName(String),
    target(name: String),
    product(name: String, package: String),
}

public enum Resource {
    copy(String),
    process(String),
}

public enum BuildSetting {
    define(String, BuildSettingCondition? = nil),
    unsafeFlags([String], BuildSettingCondition? = nil),
    linkedLibrary(String, BuildSettingCondition? = nil),
}

public struct BuildSettingCondition(
    configuration: BuildConfiguration? = nil,
    platforms: [Platform] = [],
)

public enum BuildConfiguration {
    debug,
    release,
}
```

## See also

- [Manifest](../toolchain/manifest.md)
