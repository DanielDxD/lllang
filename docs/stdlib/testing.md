# Std.Testing

**Std.Testing** is the unit test harness integrated with `lang test` ([Toolchain testing](../toolchain/testing.md)).

```ll
import Std.Testing
```

## Suites and tests

```ll
@suite("Math")
public struct MathTests {
    @test("adds")
    public func testAdd() {
        @equal(2 + 2, 4)
    }

    @theory("sum")
    @values(1, 2)
    @values(3, 4)
    public func testSum(a: Int, b: Int) {
        @expect(a + b == a + b)
    }
}

@suite("Ordered", parallel: false)
public struct OrderedTests {
    @test
    public func stepOne() { @true(true) }
}
```

- Suites and tests run **in parallel by default**. Use `parallel: false` on `@suite` for ordered cases.
- `@test` / `@theory` titles are optional (method name is used).
- Free-function `@Test` / `@test` is still supported for single cases.

## Assertion macros

Statement-prefix macros desugar to runtime checks (failure messages include the macro name):

| Macro | Meaning |
|-------|---------|
| `@expect(cond)` | `cond` is true |
| `@true(x)` / `@false(x)` | Boolean equality |
| `@equal(a, b)` / `@notEqual(a, b)` | `Equatable` / inequality |
| `@empty(x)` / `@notEmpty(x)` | `isEmpty` (alias `@noempty`) |
| `@gt` / `@gte` / `@lt` / `@lte` | Numeric comparisons |
| `@contains(hay, needle)` | String contains |
| `@approx(a, b)` | Double within default epsilon |

Free functions `expect(...)`, `expectEqual(...)`, etc. remain valid.

`@throws(E) { ... }` is documented for a later pass.

## Theories

```ll
@theory("pairs")
@values(10, 4)
@values(5, 2)
public func testSum(a: Int, b: Int) {
    @equal(a + b, a + b)
}
```

Each `@values` row becomes a concrete parallel test case.

## CLI

```bash
lang test                 # parallel by default
lang test add             # filter
lang test --jobs 4
lang test --sequential
```

Compile artifacts cache under `.lang/cache/test/<hash>/`.

## See also

- [Toolchain testing](../toolchain/testing.md)
- [Attributes](../language/14-attributes.md)
