namespace ParallelRun

import Std.IO
import Std.Async

func workerA() {
    print("A")
}

func workerB() {
    print("B")
}

func workerC() {
    print("C")
}

public func main() {
    let wg = WaitGroup.create()
    wg.add(3)
    @run workerA()
    @run workerB()
    @run workerC()
    // Give workers a moment; WaitGroup.done is left for a fuller API.
    Process.sleepMillis(50)
    let ch = Channel.create(4)
    ch.send(42)
    let v = ch.receive()
    print("ch=\(v)")
    ch.close()
    wg.done()
    wg.done()
    wg.done()
    wg.wait()
    print("done")
}
