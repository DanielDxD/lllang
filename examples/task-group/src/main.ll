namespace TaskGroupEx

import Std.IO
import Std.Async

func workerA() {
    print("A")
}

func workerB() {
    print("B")
}

public func main() {
    let g = TaskGroup()
    g.add(workerA)
    g.add(workerB)
    g.wait()
    print("done")
}
