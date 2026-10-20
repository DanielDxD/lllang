namespace ShapesOop

import Std.IO

public protocol Drawable {
    func draw() -> String
    func label() -> String {
        draw()
    }
}

public struct Circle(radius: Double): Drawable {
    public func draw() -> String {
        "circle r=\(radius)"
    }
}

public class Polygon(public let sides: Int, public let sideLength: Double): Drawable {
    public virtual func draw() -> String {
        "polygon sides=\(sides) len=\(sideLength)"
    }
}

public class ShapeBase: Drawable {
    public virtual func draw() -> String {
        "shape"
    }
}

public class Square(side: Double): ShapeBase {
    public override func draw() -> String {
        "square side=\(side)"
    }
}

public func main() {
    let circle = Circle(2.5)
    let pentagon = Polygon(sides: 5, sideLength: 3.0)
    let square = Square(4.0)

    print(circle.label())
    print(pentagon.label())
    print(square.label())
}
