namespace InteropC

import Std.IO

@extern(cName: "Mathic_add")
public func Mathic_add(a: Int, b: Int) -> Int

public func main() {
    unsafe {
        print("\(Mathic_add(2, 40))")
    }
}
