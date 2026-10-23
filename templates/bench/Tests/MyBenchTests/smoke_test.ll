namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}}")
public struct SmokeTests {
    @test("twice")
    public func testTwice() {
        @equal(twice(21), 42)
    }

    @test("fib")
    public func testFib() {
        @equal(fib(5), 5)
    }
}
