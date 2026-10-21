namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}} smoke")
public struct SmokeTests {
    @test("answer is 42")
    public func testAnswer() {
        @equal(answer(), 42)
        @expect(answer() == 42)
    }
}
