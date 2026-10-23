namespace ActorsCancel

import Std.IO
import Std.Async

public actor Counter(start: Int) {
    public func value() -> Int {
        start
    }

    public func bump(by: Int) -> Int {
        start + by
    }
}

public func backgroundWork() {
    print("bg")
}

public async func main() {
    let token = CancellationToken()
    token.throwIfCancelled()

    let c = Counter(start: 0)
    print("counter=\(c.value())")
    // Actor hop: Int args are Sendable.
    print("bumped=\(c.bump(by: 1))")

    @run backgroundWork()
    MainActor.drain()

    token.cancel()
    print("cancelled")
}
