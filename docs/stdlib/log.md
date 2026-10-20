# Std.Log

**Std.Log** provides leveled, structured logging with pluggable sinks.

```ll
import Std.Log
```

## Levels

```ll
public enum Level: Int, Comparable {
    case trace, debug, info, warn, error, fatal
}
```

## Logging

```ll
public enum Log {
    public static var level: Level { get set }
    public static func trace(_ message: String, fields: [String: LogValue] = [:])
    public static func debug(_ message: String, fields: [String: LogValue] = [:])
    public static func info(_ message: String, fields: [String: LogValue] = [:])
    public static func warn(_ message: String, fields: [String: LogValue] = [:])
    public static func error(_ message: String, fields: [String: LogValue] = [:])
}
```

```ll
public enum LogValue: Encodable {
    case string(String)
    case int(Int64)
    case bool(Bool)
    case double(Float64)
}
```

## Logger instances

```ll
public struct Logger {
    public init(subsystem: String, category: String)
    public func info(_ message: String, fields: [String: LogValue] = [:])
}
```

## Sinks

```ll
public protocol LogSink {
    func emit(_ record: LogRecord)
}

public struct LogRecord {
    public var timestamp: Instant
    public var level: Level
    public var message: String
    public var fields: [String: LogValue]
}

public enum LogConfig {
    public static func useConsole(minLevel: Level = .info)
    public static func useJsonLines(to writer: Writer, minLevel: Level = .debug)
    public static func add(_ sink: LogSink)
}
```

## Task-local context

```ll
public func withLogContext(_ fields: [String: LogValue], _ body: () async throws(E) -> Void) async throws(E)
```

Injected fields attach to all records emitted in the async scope.

## See also

- [Json](json.md) — JSON Lines encoding
- Toolchain: CLI `--verbose` / `--quiet` map to default levels
