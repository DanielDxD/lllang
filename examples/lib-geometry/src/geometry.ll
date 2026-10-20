namespace Lib.Geometry

public struct Point(x: Double, y: Double): Codable, Equatable {
    public func translated(dx: Double, dy: Double) -> Point {
        Point(x + dx, y + dy)
    }
}

public struct Rect(origin: Point, width: Double, height: Double): Codable, Equatable {
    public var center: Point => Point(origin.x + width / 2, origin.y + height / 2)

    public func area() -> Double {
        width * height
    }

    public func contains(p: Point) -> Bool {
        p.x >= origin.x && p.y >= origin.y
            && p.x <= origin.x + width && p.y <= origin.y + height
    }
}

public func union(a: Rect, b: Rect) -> Rect {
    let minX = min(a.origin.x, b.origin.x)
    let minY = min(a.origin.y, b.origin.y)
    let maxX = max(a.origin.x + a.width, b.origin.x + b.width)
    let maxY = max(a.origin.y + a.height, b.origin.y + b.height)
    Rect(Point(minX, minY), maxX - minX, maxY - minY)
}
