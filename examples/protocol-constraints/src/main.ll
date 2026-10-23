namespace ProtocolConstraints

import Std.IO

public protocol Describable {
    func describe() -> String
}

public struct Point(x: Int, y: Int): Describable {
    public func describe() -> String {
        "(\(x), \(y))"
    }
}

public struct Label(text: String): Describable {
    public func describe() -> String {
        text
    }
}

public func show<T: Describable>(value: T) -> String {
    value.describe()
}

public func showWhere<T>(value: T) where T: Describable -> String {
    value.describe()
}

public func main() {
    print(show(Point(x: 1, y: 2)))
    print(showWhere(Label(text: "hi")))
}
