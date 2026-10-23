namespace Existentials

import Std.IO

public protocol Greeter {
    func hello() -> String
}

public class Person(name: String): Greeter {
    public func hello() -> String {
        "hi \(name)"
    }
}

public func main() {
    let g: Greeter = Person("Ada")
    print(g.hello())
}
