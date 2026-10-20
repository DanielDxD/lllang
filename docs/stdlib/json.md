# Std.Json

**Std.Json** implements JSON encoding and decoding with **SIMD-accelerated scanning**, **streaming** APIs, and **zero-copy** views into input buffers where lifetimes allow.

```ll
import Std.Json
```

## Typed encode/decode

```ll
public enum JSON {
    public static func encode<T: Encodable>(_ value: T, options: EncodeOptions = []) throws(EncodingError) -> Data
    public static func decode<T: Decodable>(_ data: Data, _ type: T.Type, limits: DecoderLimits = .default) throws(DecodingError) -> T
    public static func decodeFile<T: Decodable>(_ path: Path, _ type: T.Type) throws(DecodingError) -> T
}
```

```ll
public struct EncodeOptions: OptionSet {
    public static let pretty: EncodeOptions
    public static let sortedKeys: EncodeOptions
    public static let escapeSlash: EncodeOptions
}
```

## Streaming encode

Avoid building a full in-memory tree for large values:

```ll
public struct JsonWriter {
    public init(to writer: Writer)
    public mutating func beginObject()
    public mutating func key(_ name: String)
    public mutating func value(_ string: String) throws(EncodingError)
    public mutating func value(_ number: Int64) throws(EncodingError)
    public mutating func endObject() throws(EncodingError)
}

public extension JSON {
    public static func encode<T: Encodable>(_ value: T, to writer: Writer, options: EncodeOptions = []) throws(EncodingError)
}
```

## Streaming decode

Event-driven parse for filters and partial binding:

```ll
public enum JsonEvent {
    case objectStart, objectEnd
    case arrayStart, arrayEnd
    case key(String)
    case string(String)
    case number(JsonNumber)
    case bool(Bool)
    case null
}

public struct JsonStreamParser {
    public init(limits: DecoderLimits = .default)
    public mutating func feed(_ chunk: Span<UInt8>) throws(DecodingError)
    public mutating func nextEvent() throws(DecodingError) -> JsonEvent?
}
```

## SIMD scanner

The lexer uses vectorized whitespace and string scanning on supported targets:

```ll
public struct JsonScanner {
    public init()
    public mutating func reset(input: Span<UInt8>)
    public mutating func nextToken() throws(DecodingError) -> JsonToken
}
```

Hot paths (string unescape, number parse) share kernels with `Std.Text` where profitable.

## Zero-copy strings

When decoding into `JsonValue` or using borrowed mode, string payloads can reference the source buffer:

```ll
public struct BorrowedJsonDocument {
    public init parsing data: Data, mode: BorrowMode = .copyStrings) throws(DecodingError)
    public func root() -> JsonValueRef
}

public enum BorrowMode {
    case copyStrings
    case borrowUtf8IfValid
}
```

`JsonValueRef` lifetimes tie to `BorrowedJsonDocument` — do not escape refs past document deallocation.

## Incremental HTTP bodies

```ll
public struct JsonDecoderStream<T: Decodable> {
    public init(_ type: T.Type, limits: DecoderLimits = .default)
    public mutating func append(_ chunk: Span<UInt8>) throws(DecodingError)
    public mutating func finish() throws(DecodingError) -> T
}
```

## Pretty printing and canonical forms

```ll
public func canonicalJson(_ data: Data) throws(DecodingError) -> Data
```

Useful for hashing and test fixtures (sorted keys, minimal whitespace).

## Performance guidelines

| Scenario | API |
|----------|-----|
| Small structs | `JSON.encode` / `JSON.decode` |
| Large arrays | `JsonWriter` + streaming |
| Unknown shape first | `BorrowedJsonDocument` + `JsonValueRef` |
| NDJSON logs | `JsonStreamParser` per line |
| Untrusted network | Always set `DecoderLimits` |

## See also

- [Codable](codable.md) — protocol layer
- [Toon](toon.md) — JSON round-trip via TOON for agent context
