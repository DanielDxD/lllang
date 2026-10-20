# Std.FFI

**Std.FFI** wraps C ABI interop patterns on top of language **`unsafe`** and **`@extern`** ([Interop, FFI & unsafe](../language/16-interop-ffi-unsafe.md)).

```ll
import Std.FFI
```

## C strings

```ll
public enum CString {
    public static func fromLL(_ string: String) throws(FFIError) -> Pointer<CChar>
    public static func toLL(_ cString: Pointer<CChar>) -> String
    public static func withTemporary(_ string: String, _ body: (Pointer<CChar>) throws(E) -> Void) throws(E)
}
```

## Allocators matching libc

```ll
public enum CAlloc {
    public static func malloc(_ size: Int) -> Pointer<UInt8>?
    public static func free(_ pointer: Pointer<UInt8>?)
    public static func calloc(_ count: Int, _ size: Int) -> Pointer<UInt8>?
}
```

Prefer arenas ([Memory](memory.md)) when allocation pattern is scoped.

## Span and pointer bridges

```ll
public extension Data {
    public func withUnsafeMutableBytes<R>(_ body: (MutablePointer<UInt8>, Int) throws(E) -> R) throws(E) -> R
}

public extension Span {
    public var baseAddress: Pointer<T>? { get }
}
```

## Loaded libraries

```ll
public final class DynamicLibrary {
    public static func open(_ path: Path) throws(FFIError) -> DynamicLibrary
    public func symbol(_ name: String) throws(FFIError) -> UnsafeRawPointer
}
```

Manifest `[target.*] link` remains the preferred static linking path; dynamic load is for plugins and probes.

## Callbacks (C calling LLLang)

```ll
public struct CCallback<Context, Args, Result> {
    public init(context: Context, fn: @escaping (Context, Args) -> Result)
    public func asNativePointer() -> UnsafeRawPointer
}
```

Trampoline lifetimes must outlive the registration window — document ownership in plugin APIs.

## Layout helpers

```ll
public func sizeOf<T>(_ type: T.Type) -> Int
public func alignOf<T>(_ type: T.Type) -> Int
```

Works with `@CLayout` types from the language guide.

## See also

- [Memory](memory.md) — arena-backed FFI buffers
- [Process](process.md) — `dlopen` equivalent workflows on each OS
