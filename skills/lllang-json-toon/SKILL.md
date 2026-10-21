---
name: lllang-json-toon
description: Encode and decode JSON and TOON in LLLang via Codable, Std.Json, and Std.Toon. Use for APIs, config, agent context, and .mll metadata.
---

# JSON and TOON

## Codable

- Adopt `Codable` (or manual `Encodable`/`Decodable`) on structs/enums with Codable fields.
- Compiler synthesizes when all stored properties are Codable and no `@Codable.Ignore` blocks it.
- Wire tweaks: `@Codable.Key("name")`, `@Codable.Default(x)`, `@Codable.Ignore`.
- JSON `null` → `none` for `T?`; missing keys → `none` or default — never `null` for non-optionals.

## APIs

```ll
import Std.Json
import Std.Toon

let bytes = try JSON.encode(value)
let v: Model = try JSON.decode(bytes)
let compact = TOON.encode(value)
let back: Model = try TOON.decode(compact)
```

- Prefer **JSON** for HTTP/config interchange.
- Prefer **TOON** for LLM context, `.mll` files, and token-budget-sensitive payloads.

## Enums on the wire

Tagged unions default to `{ "tag": "variant", ...fields }`. Use `@Codable.EnumEncoding(.flat)` when documented for a type.

## Safety

- Set decode limits for untrusted input (`DecoderLimits` on `JSON.decode`).
- Do not treat `.mll` or TOON blobs as authoritative types — reconcile with `.sbl`/source.

MVP status: `examples/json-toon` builds and runs (`JSON.decode(String)`, TOON round-trip, `Equatable` for `==`). Streaming/SIMD scanners remain later polish.

Docs: [docs/language/17-codable.md](../../docs/language/17-codable.md), [docs/formats/mll.md](../../docs/formats/mll.md). Example: [examples/json-toon](../../examples/json-toon).
