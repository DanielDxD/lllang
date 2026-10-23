namespace NestedCodable

import Std.IO
import Std.Json

public struct Money(amount: Int, currency: String): Codable
public struct Item(sku: String, price: Money): Codable

public func main() throws(DecodingError) {
    let json = "{\"sku\":\"X\",\"price\":{\"amount\":100,\"currency\":\"USD\"}}"
    let item: Item = try JSON.decode(json)
    let enc = JSON.encode(item)
    let back: Item = try JSON.decode(enc)
    print(back.sku)
    print(enc)
}
