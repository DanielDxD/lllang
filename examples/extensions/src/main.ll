namespace Extensions

import Std.IO

public extension String {
    public func log() {
        print(self)
    }
}

public extension Array where Element: Comparable {
    public func describeOk() {
        print("comparable-array-ok")
    }
}

public struct Point(x: Int, y: Int) {
    public var doubled: Int => self.x
    public var constSix: Int {
        get { return 6 }
    }
}

public func main() {
    "Hello, World!".log()
    var n = 1
    n += 2
    print("n=\(n)")
    for i in 1...3 {
        print("i=\(i)")
    }
    let x = 10 as Int
    print("cast=\(x)")
    if x is Int {
        print("isInt=true")
    }
    let nums: [Int] = [1, 2, 3]
    nums.describeOk()
    let p = Point(x: 3, y: 4)
    let d = p.doubled
    let s = p.constSix
    print("doubled=\(d)")
    print("six=\(s)")
}
