# Std.Env

**Std.Env** reads process environment variables and command-line arguments.

```ll
import Std.Env
```

## Environment variables

```ll
public enum Environment {
    public static func get(_ name: String) -> String?
    public static func getOrDefault(_ name: String, default: String) -> String
    public static func set(_ name: String, _ value: String) throws(EnvError)
    public static func unset(_ name: String) throws(EnvError)
    public static var keys: any Sequence<String> { get }
}
```

`set` / `unset` affect the current process only — not the parent shell.

## Command-line arguments

```ll
public enum Args {
    public static var all: [String] { get }
    public static func programName() -> String
}
```

For structured CLI parsing prefer [CLI](cli.md).

## Platform paths

```ll
public enum KnownPaths {
    public static func homeDirectory() throws(EnvError) -> Path
    public static func configDirectory(appName: String) throws(EnvError) -> Path
    public static func cacheDirectory(appName: String) throws(EnvError) -> Path
}
```

## See also

- [Process](process.md) — per-child environment overrides
