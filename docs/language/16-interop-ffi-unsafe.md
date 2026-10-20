# Interop, FFI, and unsafe

Safe LLLang code cannot forge pointers, violate memory layout, or call arbitrary machine code. **`unsafe` blocks** and **`@extern`** imports opt into C ABI interop and low-level operations with explicit audit boundaries.

**Related:** [Memory model](11-memory-model.md) · [Attributes](14-attributes.md) · [Types](02-types.md)

## unsafe blocks

```ll
unsafe {
    let ptr = libc.malloc(64)
    defer { libc.free(ptr) }
    ptr.writeBytes(source, count: 64)
}
```

Nested unsafe is allowed. Safe functions may call unsafe only inside their own unsafe regions or when marked `unsafe func`.

## unsafe functions

```ll
public unsafe func fromRawParts(ptr: Pointer<UInt8>, len: Int) -> Data {
    Data(adopting: ptr, count: len)
}
```

Callers must invoke with `unsafe` prefix at call site when crossing into unsafe API:

```ll
unsafe {
    let d = fromRawParts(ptr, len)
}
```

## Pointer types

| Type | Meaning |
|------|---------|
| `Pointer<T>` | Typed raw pointer |
| `MutablePointer<T>` | Mutable typed raw pointer |
| `UnsafePointer<T>` | Alias with stricter opt-in |
| `CChar`, `CInt` | C ABI scalars |

Optionals represent nullable C pointers: `Pointer<T>?`.

## Extern declarations

Import C functions by symbol:

```ll
@extern("c", "strlen")
public func strlen(_ s: Pointer<CChar>) -> Int

@extern("c", "printf")
public func printf(_ format: Pointer<CChar>, ...) -> Int
```

Library linkage comes from manifest `[target.linux] link = ["m"]` etc.

## Struct layout and C interop

```ll
@CLayout
public struct Vec3 {
    public var x: Float32
    public var y: Float32
    public var z: Float32
}
```

Field order and padding match platform C ABI. `@Packed` disables padding where supported.

## Fixed buffers and spans

```ll
public func checksum(span: Span<UInt8>) -> UInt32
```

`Span<T>` is bounds-checked in safe code; `.unsafeAddress` exposes pointer in unsafe contexts.

## Calling convention

Default `@extern("c")` uses C calling convention. Platform-specific attributes exist for stdcall on Windows when needed.

## Memory ownership

Document ownership at FFI boundaries:

```ll
/// Returns pointer owned by callee; do not free.
@extern("c", "get_version")
public func getVersionString() -> Pointer<CChar>

/// Transfers ownership to caller; must free with freeString.
@extern("c", "make_string")
public func makeString() -> Pointer<CChar>
```

Wrappers in safe modules convert to `String` and free native memory in `defer`.

## Error handling at boundaries

C errno / return codes map to `Result` or throws in import shims:

```ll
public func open(path: String) throws(IoError) -> File {
    unsafe {
        let fd = libc.open(path.cString, O_RDONLY)
        if fd < 0 { throw IoError.fromErrno() }
        return File(fd: fd)
    }
}
```

## Managed ↔ native pinning

When passing managed buffers to C, pin with scoped guards:

```ll
unsafe {
    data.withUnsafeBytes { bytes in
        write(fd, bytes.base, bytes.count)
    }
}
```

## Thread safety

FFI calls do not automatically hop actors. Document thread requirements of underlying libraries. `@MainActor` UI code should marshal results back after background FFI.

## WASM and other targets

`wasm32` imports use module/name attributes:

```ll
@extern("wasm", "env", "log_i32")
func logI32(value: Int32)
```

## Safe wrappers pattern

Application code should depend on safe facades:

```ll
// Unsafe in std package
namespace Std.C

// Safe in app
import Std.C as C
public func loadConfig(path: Path) throws { ... }
```

## Prohibited in safe code

- Pointer arithmetic without unsafe
- Union type punning except via std intrinsics
- Reading uninitialized memory
- Calling `@extern` without unsafe wrapper (compiler enforces)

## Summary

| Mechanism | Purpose |
|-----------|---------|
| `unsafe { }` | Lexical unsafe region |
| `@extern` | C/linker symbol import |
| `@CLayout` | ABI-stable structs |
| `Pointer<T>?` | Nullable C pointers |

Next: [Codable](17-codable.md) for JSON and TOON.
