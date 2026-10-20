# Attributes

Attributes attach metadata to declarations, parameters, and occasionally expressions. They influence availability, concurrency isolation, serialization, FFI linkage, and tooling (`.mll` generation).

**Related:** [Concurrency](12-concurrency.md) · [Interop, FFI & unsafe](16-interop-ffi-unsafe.md) · [Codable](17-codable.md)

## Syntax

Attributes prefix declarations with `@Name` or `@Name(arguments)`:

```ll
@MainActor
public class AppModel { }

@available(windows, introduced: 1.2)
public func shellNotify() { }

@deprecated("Use fetchV2 instead", removed: 2.0)
public func fetch() { }
```

Multiple attributes stack:

```ll
@Serializable
@MainActor
public struct ViewState { }
```

## Built-in attributes (core language)

| Attribute | Target | Effect |
|-----------|--------|--------|
| `@MainActor` | class, func, var | Isolate on main actor |
| `@Sendable` | func, closure type | Mark sendable function value |
| `@available(platform, introduced:)` | most | Conditional compilation / API surface |
| `@deprecated(msg, removed:)` | most | Compiler warning |
| `@inline(__always)` | func | Codegen hint |
| `@extern("symbol")` | func | FFI name (see interop) |
| `@unchecked Sendable` | class | Opt out of sendable checking |

Platform tokens include `macos`, `linux`, `windows`, `ios`, `android`, `wasm`.

## Serialization attributes

```ll
@Codable.Key("user_id")
public let userId: UUID

@Codable.Ignore
public var cache: Data?

@Codable.Default(0)
public var retryCount: Int
```

See [Codable](17-codable.md) for encoding rules.

## Testing and diagnostics

```ll
@Test
func addsItem() { ... }

@Diagnostic("unused", ignore: true)
private func legacyHook() { }
```

Test discovery integrates with `lang test`.

## LLM and tooling

```ll
@mll(summary: "Checkout service", tools: ["inventory.lookup"])
public class CheckoutService { }
```

The `lang llm gen` command reads these into `.mll` TOON packs.

## Parameter attributes

```ll
func observe(@escaping handler: () -> Void) { }
func fetch(@timeout .seconds(30)) async -> Data { }
```

Parameter-only attributes do not propagate to callers.

## Unsafe and FFI

```ll
@extern("c", "printf")
public func printf(format: UnsafePointer<CChar>, ...) -> Int

@noreturn
public func fatalError(_ message: String) -> Never
```

## Conditional attribute availability

Attributes respect `#if`:

```ll
#if DEBUG
@LogCalls
#endif
public func compute() { }
```

## Custom attributes (future)

User-defined attributes will target meta-programming and source generators:

```ll
@derive(Equatable, Hashable)
public struct Point(x: Double, y: Double)
```

Phase 1 tooling ignores unknown attributes with a warning unless registered in `lang.toml`.

## Attribute placement

Standard placement is immediately before the declaration. Members inside types place attributes before each member:

```ll
public class Example {
    @Published
    public var count: Int = 0
}
```

## Interaction with access control

`public` API with `@deprecated` still exports until removal version. `@available` may hide symbols from importers on unsupported platforms.

## Summary

- `@` prefix on declarations.
- Common: `@MainActor`, `@available`, `@deprecated`, Codable field attrs, `@extern`.
- Tooling attributes feed LLM metadata.

Next: [Namespaces & modules](15-namespaces-modules.md).
