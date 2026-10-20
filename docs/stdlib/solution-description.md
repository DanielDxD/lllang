# `Std.SolutionDescription`

API surface evaluated when loading **`Solution.ll`**. Analogous to a .NET solution file, expressed as LLLang.

```ll
namespace Std.SolutionDescription

public struct Solution(
    name: String,
    projects: [SolutionProject],
    defaultProjects: [String] = [],
    configurations: [BuildConfiguration] = [.debug, .release],
)

public enum SolutionProject {
    package(path: String, name: String? = nil),
}

public enum BuildConfiguration {
    debug,
    release,
    custom(String),
}

extension SolutionProject {
    public static func package(path: String) -> SolutionProject
    public static func package(path: String, name: String) -> SolutionProject
}
```

## Semantics

- Each `package(path:)` must contain a `Package.ll`
- `defaultProjects` match either a member package `name` from `Package.ll` or an optional solution-level `name:` override
- Configurations map to `lang build -c debug|release`

## See also

- [Solutions](../toolchain/solutions.md)
- [Manifest](../toolchain/manifest.md)
