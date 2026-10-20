namespace Shop.Core

public struct Point(x: Double, y: Double): Equatable {
    public func distance(to: Point) -> Double {
        let dx = x - to.x
        let dy = y - to.y
        let sum = dx * dx + dy * dy
        sum.squareRoot()
    }
}
