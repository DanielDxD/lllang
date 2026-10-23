namespace DeprecatedSmoke

import Std.IO

#deprecated "use greeter instead"
public func oldHello() {
    print("old")
}

public func main() {
    oldHello()
}
