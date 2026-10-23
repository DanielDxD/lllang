# Std.Text

**Std.Text** extends the built-in `String` type with encoding, parsing, searching, and formatting. Strings are immutable UTF-8 with COW backing (see [Memory model](../language/11-memory-model.md)).

```ll
import Std.Text
```

**MVP (Phase 1.1c):** `uppercased` / `lowercased` / `replacing` / `contains` / `starts(with:)` / `split` and array `joined`. Grapheme / Unicode scalar views, full `Regex`, and `some Sequence` join APIs are **Later**.

## Views and iteration

UTF-8 / grapheme views (`bytes` / `chars` / `graphemes`) are **Later**. Indexing today uses integer offsets on the UTF-8 storage.

```ll
public extension String {
    public func substring(_ range: Range<Int>) -> String
}
```

## Building and joining

```ll
// Prefer array join MVP:
// ["a", "b"].joined(separator: ",")

public struct StringBuilder {
    public init(capacity: Int = 0)
    public mutating func append(_ s: String)
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
    public func split(separator: String) -> [String]
    public func uppercased() -> String
    public func lowercased() -> String
    public func replacing(_ old: String, with new: String) -> String
}
```

## Regular expressions

`Regex` is **Later** (not shipped). Prefer `contains` / `split` / explicit parsing for now.

## Encoding

```ll
public enum Encoding {
    case utf8, utf16LE, utf16BE, latin1
}

public func encode(_ string: String, as encoding: Encoding) throws(EncodingError) -> Data
public func decode(_ data: Data, as encoding: Encoding) throws(EncodingError) -> String
```

## Formatting

Interpolated strings in source (`"\(x)"`) are the primary formatting path. A CVarArg-style `format(_:_…)` with `any` existentials is **Later**.

## See also

- [Json](json.md) — string escaping on the wire
- [CLI](cli.md) — help text wrapping
