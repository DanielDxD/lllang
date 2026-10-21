namespace UnitTestsTests

import Std.Testing
import UnitTests

@suite("Parallel UnitTest")
public struct UnitTestParallel {
    @test("should be negative")
    public func testNegative() {
        let result = 5 - 25
        @expect(result < 0)
        @lt(result, 0)
    }

    @theory("should sum two numbers")
    @values(10, 4)
    @values(5, 2)
    public func testSum(a: Int, b: Int) {
        @equal(add(a, b), a + b)
        @gte(add(a, b), a)
    }

    @test("string emptiness")
    public func testEmpty() {
        @empty("")
        @notEmpty("")
        @contains("hello", "ell")
    }
}

@suite("Sequential", parallel: false)
public struct UnitTestSequential {
    @test("negate")
    public func testNegate() {
        @equal(negate(3), 12)
        @lte(negate(3), 0)
    }

    @test
    public func testApprox() {
        @approx(1.0, 1.0)
    }
}
