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

public struct Polygon(sides: Int, sideLength: Double): Drawable {
    public func draw() -> String {
        "polygon sides=\(sides) len=\(sideLength)"
    }
}

public struct Square(side: Double): Drawable {
    public func draw() -> String {
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
