# Std.Text

**Std.Text** extends the built-in `String` type with encoding, parsing, searching, and formatting. Strings are immutable UTF-8 with COW backing (see [Memory model](../language/11-memory-model.md)).

```ll
import Std.Text
```

## Views and iteration

```ll
public extension String {
    public var bytes: UTF8Bytes { get }
    public var chars: UnicodeScalars { get }
    public var graphemes: GraphemeClusters { get }
    public func substring(_ range: Range<Int>) -> Substring
}
```

Indexing uses **integer offsets** and view iterators — not opaque string indices.

## Building and joining

```ll
public func join(_ parts: some Sequence<String>, separator: String = "") -> String

public struct StringBuilder {
    public init(capacity: Int = 0)
    public mutating func append(_ s: String)
    public mutating func append(_ c: Unicode.Scalar)
    public func build() -> String
}
```

## Parsing numbers and booleans

```ll
public extension String {
    public func parseInt(radix: Int = 10) throws(ParseError) -> Int
    public func parseFloat() throws(ParseError) -> Float64
    public func parseDecimal() throws(ParseError) -> Decimal
    public func parseBool() throws(ParseError) -> Bool
}
```

## Search and match

```ll
public extension String {
    public func contains(_ substring: String) -> Bool
    public func starts(with prefix: String) -> Bool
    public func split(separator: String, maxSplits: Int = .max) -> [String]
    public func trim(in set: CharacterSet = .whitespaceAndNewlines) -> String
}
```

## Regular expressions

```ll
public struct Regex: Sendable {
    public init(pattern: String) throws(RegexError)
    public func firstMatch(in text: String) -> Match?
    public func matches(in text: String) -> [Match]
}
```

## Encoding

```ll
public enum Encoding {
    case utf8, utf16LE, utf16BE, latin1
}

public func encode(_ string: String, as encoding: Encoding) throws(EncodingError) -> Data
public func decode(_ data: Data, as encoding: Encoding) throws(EncodingError) -> String
```

## Formatting

```ll
public func format(_ template: String, _ args: any CVarArg...) -> String
```

Interpolated strings in source (`"\(x)"`) lower to `format` or specialized builders at compile time.

## Unicode utilities

```ll
public struct Unicode {
    public static func normalize(_ s: String, form: NormalizationForm) -> String
    public static func isValidScalar(_ scalar: Unicode.Scalar) -> Bool
}
```

## Performance notes

- Prefer `Substring` when slicing large buffers repeatedly before materializing `String`.
- `StringBuilder` avoids quadratic append on `+` chains.
- Parsing hot paths share SIMD whitespace skipping with `Std.Json` scanners where applicable.

## See also

- [Json](json.md) — string escaping on the wire
- [CLI](cli.md) — help text wrapping
