// lang-tools-version: 0.1
import Std.SolutionDescription

let solution = Solution(
    name: "{{name}}",
    projects: [
        .package(path: "packages/Core"),
        .package(path: "apps/Cli"),
    ],
)
