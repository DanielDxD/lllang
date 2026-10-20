# Testing

LLLang uses a unified **`lang test`** runner integrated with the build pipeline. Tests compile as part of the package graph and execute as native binaries (Phase 2+). Phase 1 projects may still author tests; they are not executed until codegen exists.

## Layout

```text
my-lib/
  src/
    math.ll
  tests/
    math_test.ll
```

Alternative: co-located tests in `src/` with `@Test` attribute (discouraged for large libs).

Dev dependencies (`[dependencies.dev]`) supply helpers such as `test.utils` without polluting consumers.

## Defining tests

### Attribute style

```ll
namespace Acme.Math.Tests

import Acme.Math
import Std.Test

@Test
func vec2_adds_components() {
    let a = Vec2(1, 2)
    let b = Vec2(3, 4)
    Test.expectEqual(a.adding(b), Vec2(4, 6))
}
```

### Suite style (planned)

```ll
@TestSuite("Math")
struct MathTests {
    @Test
    func distance_is_non_negative() { ... }
}
```

## Running tests

```bash
lang test                    # all tests in package
lang test vec2               # filter by name substring
lang test --parallel 4
lang test --coverage         # llvm-cov integration (Phase 3+)
```

In workspaces:

```bash
lang test --workspace
lang test -p acme.math
```

## Assertions

Via `Std.Test` (documented in stdlib):

| API | Behavior |
|-----|----------|
| `Test.expect(condition)` | Fails if false |
| `Test.expectEqual(a, b)` | Uses `Equatable` |
| `Test.expectThrows(E) { ... }` | Typed error expectation |
| `Test.snap("name", value)` | TOON/JSON snapshot files in `tests/snapshots/` |

Snapshots are reviewed in PRs; CI fails on drift unless `--accept` locally.

## Integration tests

Place under `tests/integration/` with longer timeouts:

```toml
[build]
test-timeout-ms = 120_000
```

Mark with `@Test(integration: true)` when supported.

## CI example

```yaml
- run: lang restore
- run: lang test --workspace -- --parallel 2
```

Requires lockfile committed and store cache keyed by `Package.lock` hashes.

## Relationship to `check`

Phase 1:

```bash
lang check tests/
```

Ensures test sources parse. No execution.

## Benchmarks vs tests

| | `lang test` | `lang bench` |
|---|-------------|--------------|
| Location | `tests/` | `benches/` |
| Attribute | `@Test` | `@Bench` |
| Failure | Assertion | Regression vs baseline |

See [CLI `bench`](cli-reference.md).

## See also

- [Templates: test](templates.md)
- [Build pipeline](build-pipeline.md)
- [Std.Test](../stdlib/overview.md) (API surface)
