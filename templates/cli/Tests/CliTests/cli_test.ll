namespace {{Name}}Tests

import Std.Testing
import {{Name}}

@suite("{{Name}} cli")
public struct CliTests {
    @test("usage mentions help")
    public func testUsage() {
        @contains(usage(), "--help")
    }

    @test("version string")
    public func testVersion() {
        @contains(version(), "0.1.0")
    }

    @test("parse modes")
    public func testParse() {
        @equal(parseMode(1, ""), 3)
        @equal(parseMode(2, "--help"), 0)
        @equal(parseMode(2, "--version"), 1)
        @equal(parseMode(2, "Ada"), 2)
    }

    @test("greet")
    public func testGreet() {
        @equal(greet("Ada"), "Hello, Ada!")
    }
}
