# Std.Toon

**Std.Toon** implements **TOON (Token-Oriented Object Notation)** — a line-oriented, indentation-scoped format optimized for LLM context and `.mll` metadata. JSON remains the interchange lingua franca; TOON emphasizes compactness and tabular arrays.

```ll
import Std.Toon
```

Format rationale: [ADR 004](../design/adr/004-toon-for-mll.md) · [`.mll` spec](../formats/mll.md).

## Encode and decode

```ll
public enum TOON {
    public static func encode<T: Encodable>(_ value: T, options: EncodeOptions = []) throws(EncodingError) -> String
    public static func decode<T: Decodable>(_ text: String, _ type: T.Type, limits: DecoderLimits = .default) throws(DecodingError) -> T
    public static func encodeToData<T: Encodable>(_ value: T) throws(EncodingError) -> Data
}
```

```ll
public struct EncodeOptions {
    public var indent: Int = 2
    public var tabularThreshold: Int = 8
    public var preferInlineArrays: Bool = false
}
```

## Tabular arrays

Homogeneous object arrays collapse to columnar layout when `tabularThreshold` is met:

```ll
public struct Row {
    public let id: Int
    public let name: String
}

let rows = [Row(1, "a"), Row(2, "b"), Row(3, "c")]
let toon = TOON.encode(rows, options: EncodeOptions(tabularThreshold: 2))
```

Example wire shape (conceptual):

```toon
rows[id,name]:
  1,a
  2,b
  3,c
```

Decoder restores `[Row]` with the same field order declared in the header.

## JSON round-trip

Convert without losing structure for agent pipelines:

```ll
public enum TOON {
    public static func fromJson(_ json: Data) throws(DecodingError) -> String
    public static func toJson(_ toon: String, pretty: Bool = false) throws(DecodingError) -> Data
}
```

Round-trip preserves:

- Object key order as encountered (unless `sortedKeys` requested on JSON export)
- Number string forms that JSON and TOON both represent exactly
- `null` vs missing key distinctions per `DecoderLimits` policy

Round-trip may **not** preserve redundant whitespace or comment lines (TOON has no comment syntax in v1).

## Streaming

```ll
public struct ToonWriter {
    public init(to writer: Writer, options: EncodeOptions = EncodeOptions())
    public mutating func write<T: Encodable>(_ value: T) throws(EncodingError)
}

public struct ToonParser {
    public init(limits: DecoderLimits = .default)
    public mutating func parse<T: Decodable>(_ input: Span<UInt8>, _ type: T.Type) throws(DecodingError) -> T
}
```

## Merge and patch (context assembly)

Used by `lang llm context` when combining lockfile, `.mll`, and snippets:

```ll
public func mergeToonDocuments(_ base: String, _ overlay: String) throws(DecodingError) -> String
```

Overlay keys win; arrays append unless keyed by `id` fields in schema hints.

## Strict indentation

Parsers reject inconsistent indent width and mixed tabs/spaces — fail closed like YAML strict mode to protect agents from silent structure drift.

## See also

- [Json](json.md) — SIMD JSON for bulk interchange
- [Llm](../llm/context-generation.md) — context bundles
- Language: [Codable](../language/17-codable.md)
