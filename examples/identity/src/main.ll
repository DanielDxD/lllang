namespace IdentityExample

import Std.IO
import Std.Identity

public func main() {
    let uuid = Identity.uuid()
    let ulid = Identity.ulid()
    let nanoid = Identity.nanoid()

    print("uuid=\(uuid)")
    print("ulid=\(ulid)")
    print("nanoid=\(nanoid)")
}
