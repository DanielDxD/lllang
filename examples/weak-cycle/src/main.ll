namespace WeakCycle

import Std.IO

public class Node(name: String) {
    public weak var parent: Node?
    public var child: Node?
}

public func main() {
    let parent = Node("parent")
    let child = Node("child")
    child.parent = parent
    parent.child = child
    // Weak back-edge: cycle must not keep both objects alive forever.
    print("ok")
}
