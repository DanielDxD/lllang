namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}} smoke")
public struct SmokeTests {
    @test("twice")
    public func testTwice() {
        @equal(twice(5), 10)
    }
}
