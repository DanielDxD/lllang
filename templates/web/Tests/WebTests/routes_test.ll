namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}} routes")
public struct RoutesTests {
    @test("health")
    public func testHealth() {
        @contains(handle("/health"), "ok")
        @contains(healthBody(), "status")
    }

    @test("unknown")
    public func test404() {
        @contains(handle("/nope"), "404")
    }
}
