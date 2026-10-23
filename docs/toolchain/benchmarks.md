# Benchmarks

`lang bench` finds `@Bench` functions under `benches/**/*.ll`, builds a small harness that runs each one for a fixed iteration count, and prints wall-clock milliseconds per bench.

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

## Writing a bench

```ll
namespace MyBenchBench

@Bench
func twice10() {
    let _x = twice(10)
}
```

Bench sources are analyzed together with package library sources (same discovery as `lang check`, excluding `Tests/` and `benches/` from the lib set, then adding all `benches/*.ll`). The generated harness lives under `.lang/bench/` and is linked like a normal debug binary.

Example output:

```text
     Running 1 bench
twice10: 12 ms (200 iters)
    Finished 1 bench in 0.15s
```

## See also

- [Templates](templates.md) — `bench` scaffold
- [CLI reference](cli-reference.md) — `lang bench`
- [Testing](testing.md)
