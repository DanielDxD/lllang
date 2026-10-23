namespace UnitTests

public enum Boom: Error {
    gone,
}

public func add(a: Int, b: Int) -> Int {
    return a + b
}

public func negate(n: Int) -> Int {
    return 0 - n
}

public func boom() throws(Boom) {
    throw Boom.gone
}
