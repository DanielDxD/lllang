# Std.Codable

**Std.Codable** defines the encoder/decoder protocols and keyed containers shared by JSON, TOON, and custom formats.

```ll
import Std.Codable
```

Language-level overview: [Codable](../language/17-codable.md).

## Protocols

```ll
public protocol Encodable {
    func encode(to encoder: Encoder) throws(EncodingError)
}

public protocol Decodable {
    init(from decoder: Decoder) throws(DecodingError)
}

public protocol Codable: Encodable, Decodable {}
```

## Encoder and Decoder

```ll
public protocol Encoder {
    func container() -> KeyedContainer
    func unkeyedContainer() -> UnkeyedContainer
    func singleValue<T: Encodable>(_ value: T) throws(EncodingError)
}

public protocol Decoder {
    func container() -> KeyedDecodingContainer
    func unkeyedContainer() -> UnkeyedDecodingContainer
    func singleValue<T: Decodable>(_ type: T.Type) throws(DecodingError) -> T
}
```

## Keyed containers

```ll
public struct KeyedContainer {
    public mutating func encode<T: Encodable>(_ key: String, _ value: T) throws(EncodingError)
    public mutating func encodeIfPresent<T: Encodable>(_ key: String, _ value: T?) throws(EncodingError)
}

public struct KeyedDecodingContainer {
    public func decode<T: Decodable>(_ key: String, _ type: T.Type) throws(DecodingError) -> T
    public func decodeIfPresent<T: Decodable>(_ key: String, _ type: T.Type) throws(DecodingError) -> T?
}
```

## Limits and security

```ll
public struct DecoderLimits {
    public var maxDepth: Int = 64
    public var maxStringBytes: Int = 16 * 1024 * 1024
    public var maxContainerSize: Int = 1_000_000
}

public extension Decoder {
    public var limits: DecoderLimits { get set }
}
```

Apply limits when decoding untrusted input (HTTP bodies, agent tool payloads).

## Dynamic values

```ll
public enum JsonValue: Codable, Equatable {
    case null
    case bool(Bool)
    case number(String)
    case string(String)
    case array([JsonValue])
    case object([String: JsonValue])
}
```

## Compiler synthesis

The compiler synthesizes `Codable` when all stored properties are `Codable` and no `@Codable.Ignore` blocks synthesis. Field attributes (`@Codable.Key`, `@Codable.Default`) live in the language guide.

## See also

- [Json](json.md) — JSON-specific SIMD and streaming
- [Toon](toon.md) — TOON wire format
