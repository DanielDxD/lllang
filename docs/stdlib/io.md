# Std.IO

**Std.IO** defines byte and text streams, buffering, and standard handles.

```ll
import Std.IO
```

## Standard handles

```ll
public enum StdIO {
    public static var stdin: Reader { get }
    public static var stdout: Writer { get }
    public static var stderr: Writer { get }
}
```

## Reader and Writer

```ll
public protocol Reader {
    func read(into buffer: inout [UInt8]) throws(IOError) -> Int
    func readAll() throws(IOError) -> Data
}

public protocol Writer {
    func write(_ buffer: Span<UInt8>) throws(IOError)
    func flush() throws(IOError)
}
```

## Buffering

```ll
public struct BufferedReader<R: Reader> {
    public init(underlying: R, capacity: Int = 8192)
    public mutating func readLine() throws(IOError) -> String?
}

public struct BufferedWriter<W: Writer> {
    public init(underlying: W, capacity: Int = 8192)
    public mutating func writeLine(_ line: String) throws(IOError)
}
```

## Data and spans

```ll
public struct Data: Equatable, Hashable, Sendable {
    public init()
    public init(_ bytes: [UInt8])
    public var count: Int { get }
    public func withUnsafeBytes<R>(_ body: (Span<UInt8>) throws(E) -> R) throws(E) -> R
}
```

## Print helpers

```ll
public func print(_ items: any CustomStringConvertible..., terminator: String = "\n")
public func eprint(_ items: any CustomStringConvertible..., terminator: String = "\n")
```

## See also

- [FS](fs.md) — file-backed readers/writers
- [Json](json.md) — encode to `Writer`
