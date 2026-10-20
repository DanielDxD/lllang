# Codable

**Codable** is the protocol family for serializing values to and from external formats. JSON and **TOON** (token-oriented object notation for LLM contexts) are first-class in `Std.Json` and `Std.Toon`.

**Related:** [Structs, classes, enums](06-structs-classes-enums.md) · [Attributes](14-attributes.md) · [Optionals](09-optionals.md)

## Protocol overview

```ll
public protocol Encodable {
    func encode(to encoder: Encoder) throws(EncodingError)
}

public protocol Decodable {
    init(from decoder: Decoder) throws(DecodingError)
}

public protocol Codable: Encodable, Decodable {}
```

Adopt on types that should round-trip:

```ll
public struct Money(amount: Decimal, currency: String): Codable, Equatable
public class Order(public let id: UUID): Codable { ... }
```

## Synthesis

The compiler synthesizes `Codable` when:

- All stored properties are `Codable`
- Enums only have associated values that are `Codable`
- No `@Codable.Ignore` blocks synthesis

Manual implementation when customizing:

```ll
public struct Version: Codable {
    public let major: Int
    public let minor: Int

    public init(from decoder: Decoder) throws(DecodingError) {
        let c = try decoder.container()
        major = try c.decode("major", Int.self)
        minor = try c.decode("minor", Int.self)
    }

    public func encode(to encoder: Encoder) throws(EncodingError) {
        var c = encoder.container()
        try c.encode("major", major)
        try c.encode("minor", minor)
    }
}
```

## Field attributes

```ll
public struct User: Codable {
    @Codable.Key("user_id")
    public let id: UUID

    @Codable.Default("")
    public var nickname: String

    @Codable.Ignore
    public var sessionToken: String?
}
```

| Attribute | Effect |
|-----------|--------|
| `@Codable.Key("name")` | Wire name override |
| `@Codable.Ignore` | Skip field |
| `@Codable.Default(x)` | Missing key uses default |

## Optionals and null

JSON `null` decodes to `none` for `T?`. Missing keys decode to `none` or `@Codable.Default`. There is no null for non-optional fields — decode errors propagate as `DecodingError`.

## Enums on the wire

Tagged union encoding by default:

```ll
enum Shape: Codable {
    circle(radius: Double),
    rect(w: Double, h: Double),
}
```

JSON example:

```json
{ "tag": "circle", "radius": 1.5 }
```

Configure with `@Codable.EnumEncoding(.flat)` for alternative layouts.

## Classes and reference cycles

Classes encode by value fields; weak references are omitted. Decode reconstructs graphs when using `@Codable.Identity` on selected reference fields (advanced).

## JSON API

```ll
import Std.Json

let json = try JSON.encode(order)
let copy: Order = try JSON.decode(json)

let pretty = JSON.encode(order, options: [.pretty, .sortedKeys])
```

Streaming:

```ll
try JSON.encode(order, to: writer)
```

## TOON API

TOON favors compact, LLM-friendly token streams:

```ll
import Std.Toon

let toon = TOON.encode(order)
let restored: Order = try TOON.decode(toon)
```

Use TOON in agent tools and `.mll` context packs. Binary `.bll` metadata may embed TOON schemas.

## Custom types

```ll
public struct Date: Codable {
    public let iso: String
    public init(from decoder: Decoder) throws(DecodingError) {
        iso = try decoder.singleValue(String.self)
    }
    public func encode(to encoder: Encoder) throws(EncodingError) {
        try encoder.singleValue(iso)
    }
}
```

## Key paths and dynamic keys

```ll
let patch: [String: JsonValue] = try JSON.decode(raw)
```

`JsonValue` enum supports dynamic documents alongside typed models.

## Versioning

Evolve schemas with optional fields and defaults:

```ll
public struct Config: Codable {
    public var timeout: Duration = .seconds(30)   // new field
    public var retries: Int = 3
}
```

Breaking renames use `@Codable.Key` or migration decoders.

## Security

- Do not decode untrusted types with `@Codable.Identity` graph restore.
- Cap decode depth and payload size via `DecoderLimits` in `JSON.decode`.
- Validate enums — unknown tags fail closed unless `@Codable.UnknownCase`.

## Performance

Encoding uses static serializers generated at compile time for synthesized types. Large collections stream; avoid materializing entire DOM for huge payloads.

## Testing round-trip

```ll
@Test
func moneyRoundTrip() {
    let m = Money(10.5d, "USD")
    let bytes = try JSON.encode(m)
    let m2: Money = try JSON.decode(bytes)
    expect(m2 == m)
}
```

## Summary

| Format | Package | Typical use |
|--------|---------|-------------|
| JSON | `Std.Json` | APIs, config |
| TOON | `Std.Toon` | LLM tools, `.mll` |
| Protocol | `Codable` | Shared encode/decode |

Return to the [documentation index](../index.md) or review [Error handling](10-error-handling.md) for decode failures.
