namespace IdentitySmoke

import Std.IO
import Std.Identity

public func main() {
    let uuid = Identity.uuid()
    let ulid = Identity.ulid()
    let nanoid = Identity.nanoid()

    print("uuid=\(uuid)")
    print("ulid=\(ulid)")
    print("nanoid=\(nanoid)")

    let upper = uuid.uppercased()
    let lower = uuid.lowercased()
    print("upper=\(upper)")
    print("lower=\(lower)")

    let replaced = "hello-lllang".replacing("lllang", with: "world")
    print("replaced=\(replaced)")

    let parts = "a,b,c".split(",")
    let joined = parts.joined(separator: "|")
    print("joined=\(joined)")

    print("abs=\(abs(-42))")
    print("max=\(max(10, 20))")
    print("clamp=\(clamp(150, 0, 100))")
    print("sqrt=\(sqrt(144.0))")
}
