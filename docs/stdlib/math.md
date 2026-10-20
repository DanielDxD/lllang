# Std.Math

**Std.Math** provides scalar math, `Decimal` helpers, and small linear-algebra types.

```ll
import Std.Math
```

## Scalar functions

```ll
public func abs(_ x: Int) -> Int
public func abs(_ x: Float64) -> Float64
public func min<T: Comparable>(_ a: T, _ b: T) -> T
public func max<T: Comparable>(_ a: T, _ b: T) -> T
public func clamp<T: Comparable>(_ value: T, _ lo: T, _ hi: T) -> T

public func sqrt(_ x: Float64) -> Float64
public func pow(_ base: Float64, _ exp: Float64) -> Float64
public func sin(_ x: Float64) -> Float64
public func cos(_ x: Float64) -> Float64
```

## Decimal

```ll
public extension Decimal {
    public static func parse(_ text: String) throws(ParseError) -> Decimal
    public func rounded(_ scale: Int, mode: RoundingMode) -> Decimal
    public func + (lhs: Decimal, rhs: Decimal) -> Decimal
}
```

Use `Decimal` for money; avoid `Float64` for currency.

## Vectors (2D / 3D)

```ll
public struct Vec2 {
    public var x: Float32
    public var y: Float32
    public init(x: Float32, y: Float32)
    public func dot(_ other: Vec2) -> Float32
    public func length() -> Float32
}

public struct Vec3 {
    public var x: Float32
    public var y: Float32
    public var z: Float32
    public init(x: Float32, y: Float32, z: Float32)
    public func cross(_ other: Vec3) -> Vec3
}
```

## Random (non-crypto)

```ll
public struct Random {
    public init(seed: UInt64)
    public mutating func nextInt(in range: Range<Int>) -> Int
    public mutating func nextFloat() -> Float64
}
```

For security-sensitive randomness use [Crypto](crypto.md).

## See also

- [Types](../language/02-types.md) — numeric conversions
