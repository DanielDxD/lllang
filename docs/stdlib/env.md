# Std.Env

**Std.Env** reads process environment variables and command-line arguments. Prefer the API name **`Env`** (`Environment` remains an alias).

```ll
import Std.Env
```

**MVP (Phase 1.1c):** typed getters with defaults / optionals — `string`, `integer`/`int`, `bool`, `float`/`double`, `decimal`, `uint`, `int64` — plus `@loadEnv` for `.env` files. Existential `any Sequence` key iteration is **Later**.

## Environment variables

```ll
public enum Env {
    public static func get(_ name: String) -> String?
    public static func getOrDefault(_ name: String, default: String) -> String
    public static func string(_ name: String, default: String) -> String
    public static func integer(_ name: String, default: Int) -> Int
    public static func bool(_ name: String, default: Bool) -> Bool
    public static func float(_ name: String, default: Float) -> Float
    public static func double(_ name: String, default: Double) -> Double
    public static func decimal(_ name: String, default: Decimal) -> Decimal
}
```

`set` / `unset` affect the current process only — not the parent shell (when exposed).

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
- `examples/env-typed`
