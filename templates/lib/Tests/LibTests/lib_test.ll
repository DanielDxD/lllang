namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}}")
public struct LibTests {
    @test("add")
    public func testAdd() {
        @equal(add(2, 3), 5)
    }

    @test("placeholder message")
    public func testPlaceholder() {
        let p = Placeholder(message: "ok")
        @notEmpty(p.message)
        @equal(p.message, "ok")
    }
}
