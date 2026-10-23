namespace StdlibBaseline

import Std.IO
import Std.FS
import Std.Env
import Std.Json
import Std.Core
import Std.Collections
import Std.Time
import Sum

public struct Address(city: String, zip: Int): Codable
public struct Person(name: String, age: Int, active: Int, address: Address): Codable

public enum DemoError: Error {
    boom,
}

public func mayFail(ok: Bool) throws(DemoError) -> String {
    if ok {
        return "ok"
    }
    throw .boom
}

public func main() {
    let result = sum(1, 2)
    print("Result: \(result)")
}
