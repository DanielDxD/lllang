namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}}")
public struct LibTests {
    @test("placeholder message")
    public func testPlaceholder() {
        let p = Placeholder(message: "ok")
        @notEmpty(p.message)
        @equal(p.message, "ok")
    }
}
