namespace Shop.Cli

import Shop.Core
import Std.IO

public func main() {
    let a = Point(0, 0)
    let b = Point(3, 4)
    print("distance: \(a.distance(b))")
}
