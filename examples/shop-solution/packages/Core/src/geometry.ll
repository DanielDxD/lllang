namespace Shop.Core

public struct Point(x: Double, y: Double): Equatable {
    public func distance(to: Point) -> Double {
        let dx = x - to.x
        let dy = y - to.y
        (dx * dx + dy * dy).squareRoot()
    }
}
