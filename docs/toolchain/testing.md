# Testing

LLLang uses a unified **`lang test`** runner: discover suites, compile a cached harness, execute cases **in parallel by default**.

## Layout

```text
my-lib/
  Sources/...
  Tests/
    math_test.ll
```

`testTarget` paths in `Package.ll` are scanned. Files with `@suite` / `@test` / `@Test` are included.

## Defining tests

### Suite style (preferred)

```ll
namespace Acme.Math.Tests

import Std.Testing

@suite("Math")
public struct MathTests {
    @test("distance non-negative")
    public func distanceOk() {
        @gte(1, 0)
    }
}
```

### Free function

```ll
@Test
func vec2_adds() {
    @equal(1 + 1, 2)
}
```

See [Std.Testing](../stdlib/testing.md) for assertion macros (`@expect`, `@empty`, `@gte`, …) and theories.

## Running

```bash
lang test                    # all tests, parallel
lang test vec2               # filter by name substring
lang test --jobs 8
lang test --sequential       # one worker (still respects suite parallel: false)
```

Cache: `.lang/cache/test/<content-hash>/` (skip relink when sources unchanged).

## CI

```yaml
- run: lang restore
- run: lang test
```

## See also

- [Benchmarks](benchmarks.md) — `lang bench`
- [CLI reference](cli-reference.md)
