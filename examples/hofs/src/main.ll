namespace Hofs

import Std.IO

public func main() {
    let xs: [Int] = [1, 2, 3, 4, 5]
    let doubled = xs.map((x) => x * 2)
    print("\(doubled.count)")
    let evens = xs.filter((x) => x == 2 or x == 4)
    print("\(evens.count)")
    let add = 10
    let bumped = xs.map((x) => x + add)
    print("\(bumped.count)")
    let sum = xs.reduce(0, (a, x) => a + x)
    print("\(sum)")
    let anyBig = xs.any((x) => x > 4)
    if anyBig {
        print("yes")
    } else {
        print("no")
    }
    let sorted = [3, 1, 2].sorted()
    print("\(sorted.count)")
}
