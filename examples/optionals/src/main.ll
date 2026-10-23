namespace Optionals

import Std.IO

public func main() {
    let a: String? = "Ada"
    let b: String? = nil

    // Nil-coalesce
    print(a ?? "none")
    print(b ?? "guest")

    // if let
    if let name = a {
        print(name)
    } else {
        print("missing")
    }

    // Optional chaining
    let trimmed = a?.trim()
    print(trimmed ?? "empty")
    print(b?.trim() ?? "nope")

    // Force unwrap (known some)
    print(a!)
}
