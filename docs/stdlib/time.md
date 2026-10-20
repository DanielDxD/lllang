# Std.Time

**Std.Time** covers monotonic and wall-clock time, durations, and time zones.

```ll
import Std.Time
```

## Duration

```ll
public struct Duration: Equatable, Comparable, Codable {
    public static func seconds(_ value: Int64) -> Duration
    public static func milliseconds(_ value: Int64) -> Duration
    public static func nanoseconds(_ value: Int64) -> Duration
    public var totalSeconds: Float64 { get }
}
```

Literal forms like `.seconds(10)` infer `Duration` in parameter context.

## Instant and DateTime

```ll
public struct Instant: Equatable, Comparable, Codable {
    public static func now() -> Instant
    public func elapsedSince(_ earlier: Instant) -> Duration
}

public struct DateTime: Equatable, Codable {
    public var year: Int
    public var month: Int
    public var day: Int
    public var hour: Int
    public var minute: Int
    public var second: Int
    public var nanosecond: Int
    public func toInstant(in zone: TimeZone) throws(TimeError) -> Instant
}
```

## Time zones

```ll
public struct TimeZone: Sendable {
    public static let utc: TimeZone
    public static func named(_ identifier: String) throws(TimeError) -> TimeZone
}
```

## Sleep and timers

```ll
public func sleep(_ duration: Duration) async
public struct Timer {
    public init(every interval: Duration, leeway: Duration = .milliseconds(1))
    public func waitNext() async
}
```

Blocking sleep for sync code:

```ll
public func sleepBlocking(_ duration: Duration)
```

## Formatting and parsing

```ll
public struct ISO8601 {
    public static func format(_ instant: Instant, in zone: TimeZone = .utc) -> String
    public static func parse(_ text: String) throws(ParseError) -> Instant
}
```

## See also

- [Async](async.md) — `sleep` integration with tasks
- [Log](log.md) — timestamps on records
