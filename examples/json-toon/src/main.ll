namespace JsonToon

import Std.IO
import Std.Json
import Std.Toon

public struct Product(sku: String, name: String, priceCents: Int): Codable, Equatable

public func main() throws(DecodingError) {
    let json = "{\"sku\":\"LL-001\",\"name\":\"LLLang mug\",\"priceCents\":1499}"

    let product: Product = try JSON.decode(json)
    let toon = TOON.encode(product)
    let roundtrip: Product = try TOON.decode(toon)

    let status = switch product == roundtrip {
        true => "roundtrip ok",
        false => "mismatch",
    }

    print(status)
    print(toon)
}
