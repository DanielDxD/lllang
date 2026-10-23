namespace Cancellation

import Std.IO
import Std.Async

public async func cancellableFunc(cancellation: CancellationToken) {
    cancellation.throwIfCancelled()
    print("Cancellable function running")
}

public async func main() {
    let cancellationToken = CancellationToken()
    await cancellableFunc(cancellationToken)
    cancellationToken.cancel()
    print("done")
}
