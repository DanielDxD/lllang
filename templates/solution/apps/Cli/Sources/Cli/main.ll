namespace Cli

import Std.IO
import Core

public func main() {
    print(hello("LLLang"))
    print(Greeting(text: "hi").text)
    print("sum=\(sum(2, 3))")
}
