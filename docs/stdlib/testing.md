# Std.Testing

**Std.Testing** is the unit test harness integrated with `lang test` ([Toolchain testing](../toolchain/testing.md)).

```ll
import Std.Testing
```

## Tests

```ll
@Test
func addsNumbers() {
    expect(2 + 2 == 4)
}

@Test(.tags(["integration"]))
func readsConfig() throws(ConfigError) {
    let cfg = try File.readText(Path("fixture.toml"))
    expect(cfg.contains("version"))
}
```

## Expectations

```ll
public func expect(_ condition: Bool, _ message: String = "expectation failed")
public func expectEqual<T: Equatable>(_ lhs: T, _ rhs: T)
public func expectThrows<E: Error>(_ errorType: E.Type, _ body: () throws(E) -> Void)
```

## Async tests

```ll
@Test
func fetchesHealth() async throws(HttpError) {
    let client = HttpClient(base: URL("https://example.com"))
    let resp = try await client.get("/health")
    expect(resp.status == 200)
}
```

## Fixtures

```ll
public struct TestFixture<Context> {
    public init(_ setup: () throws(E) -> Context, _ teardown: (Context) -> Void = { _ in })
    public func with<R>(_ body: (Context) throws(E) -> R) throws(E) -> R
}
```

## Benchmark hook

Micro-benchmarks in tests (not a replacement for `lang bench`):

```ll
@Test(.benchmark)
func hashPerformance() {
    measure(iterations: 10_000) {
        Hasher().combine("payload")
    }
}
```

## See also

- [FS](fs.md) — temporary files in tests
- [Json](json.md) — round-trip fixtures
