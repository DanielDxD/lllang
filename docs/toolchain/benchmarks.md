# Benchmarks

Phase 4 MVP: put timed checks under `benches/**/*.ll` and run:

```bash
lang bench
```

## Layout

Use the `bench` template:

```bash
lang new bench -n mybench
```

```text
mybench/
  Sources/MyBench/lib.ll
  benches/fib.ll
  Tests/...
```

Mark harness entry points with `@Bench` (documented; the MVP harness typechecks each `benches/*.ll` and reports wall-clock check time).

## Writing a bench

```ll
namespace MyBenchBench

@Bench
func twice10() {
    let _x = twice(10)
}
```

Keep bench files self-contained (or analyze the whole package) so `lang bench` typechecks cleanly.

Full timed execution of native binaries (like Criterion-style harnesses) is future work; today `lang bench` validates that bench sources analyze cleanly and reports timing for the check itself.

## See also

- [Templates](templates.md) — `bench` scaffold
- [CLI reference](cli-reference.md) — `lang bench`
- [Testing](testing.md)
