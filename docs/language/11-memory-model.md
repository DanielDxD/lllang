# Memory model

LLLang uses **automatic reference counting (ARC)** for classes and **value semantics** for structs and enums, with copy-on-write (COW) for large value buffers. There is no tracing garbage collector in the language runtime.

**Related:** [Structs, classes, enums](06-structs-classes-enums.md) · [Concurrency](12-concurrency.md) · [Interop, FFI & unsafe](16-interop-ffi-unsafe.md)

## Value types

Structs and enums live on the stack or inline in parents unless escaped. Assignment copies data:

```ll
var a = Point(1, 2)
var b = a
b.x = 3        // a.x still 1
```

Large collections (`String`, `[T]`, `Dictionary`) use COW: mutation triggers unique copy of backing storage.

## Reference types

Classes allocate on the heap; variables hold references.

```ll
let x = Order(id)
let y = x      // shared reference; retain count +1
```

When retain count reaches zero, **`deinit` runs deterministically** on that thread (unless weak-only resurrection edge cases are avoided by design).

```ll
deinit {
    socket.close()
    Log.trace("released \(id)")
}
```

## weak and unowned

Break retain cycles between classes:

```ll
public class Customer {
    public var orders: [Order] = []
}

public class Order {
    public weak var customer: Customer?
}
```

| Qualifier | When target deallocates |
|-----------|-------------------------|
| `weak` | Reference becomes `nil` |
| `unowned` | Access traps (unsafe if outlived) |

Use `weak` for delegates and parent back-links. Use `unowned` only when the owner guarantees shorter lifetime.

```ll
public class Node {
    public unowned let parent: Graph
    public init(parent: Graph) { self.parent = parent }
}
```

## No null for references

Optional class references use `T?` — never null pointers in safe code.

## Structs containing classes

Copying a struct copies references, not deep-cloning objects:

```ll
struct Bundle {
    var order: Order   // class ref
}
var b1 = Bundle(order: o)
var b2 = b1          // same Order instance
```

Deep copy requires explicit API (`clone()`).

## Capture and closures

Closures retain captured reference types strongly by default:

```ll
self.onComplete = { [weak self] in
    self?.finish()
}
```

Capture lists: `[weak x]`, `[unowned y]`, `[let z]` for value capture.

## Deterministic cleanup

- `deinit` order for nested objects: children before parents when owned strongly.
- Global and static destructors run at process exit in reverse registration order.
- `defer` handles scope-local cleanup (see [Statements](05-statements.md)).

## Stack allocation and escape analysis

The compiler promotes non-escaping temporaries to stack slots. `@escaping` on stored function types prevents invalid stack pointers.

## Unsafe memory

Raw pointers, manual allocation, and MMIO live in `unsafe` blocks (see [Interop, FFI & unsafe](16-interop-ffi-unsafe.md)). Safe code cannot forge references from integers.

## Concurrency and memory

Reference counting uses atomic operations for cross-thread retains/releases on shared objects. Value types copied per task avoid sharing unless wrapped in a class or sync container.

`Sendable` protocol marks types safe to pass across concurrency domains (see [Concurrency](12-concurrency.md)).

## Cycles and diagnostics

Debug builds may enable cycle logging for `weak`/`unowned` misuse. Instruments hooks (future) visualize retain paths.

## Comparison with other systems

| Aspect | LLLang |
|--------|--------|
| GC | No |
| Class lifetime | ARC |
| Struct assignment | Copy (COW) |
| deinit | Deterministic |
| null | Not in safe code |

## Best practices

1. Prefer structs for data transfer; classes for identity and shared lifecycle.
2. Use `weak` for bidirectional class graphs.
3. Keep `deinit` lightweight — avoid blocking I/O.
4. Do not rely on finalization order across modules.

## Summary

- ARC for classes; value + COW for structs/enums.
- `weak` / `unowned` manage cycles.
- `deinit` runs when last strong reference goes away.

Next: [Concurrency](12-concurrency.md).
