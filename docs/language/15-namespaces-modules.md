# Namespaces and modules

A **package** is the unit of compilation, versioning, and dependency (`lang.toml`). **Namespaces** organize source within a package into logical names similar to C# namespaces, without requiring one type per file.

**Related:** [Declarations](03-declarations.md) · [Attributes](14-attributes.md)

## Packages

Each project has a manifest:

```toml
[package]
name = "shop.checkout"
version = "1.0.0"
```

Built artifacts:

| Artifact | Role |
|----------|------|
| `.bll` | Precompiled binary for a target triple |
| `.sbl` | Public type signatures for linking |
| `.ll` | Source (dev or optional source packages) |

Dependencies resolve semver via `lang add`.

## Namespaces

```ll
namespace Shop.Checkout

public struct Cart { ... }
public class CheckoutService { ... }
```

Nested namespaces flatten in metadata:

```ll
namespace Shop {
    namespace Checkout {
        public struct Cart { }
    }
}
// Full name: Shop.Checkout.Cart
```

Files in the same package may use different namespace blocks; default is root namespace from manifest `[package].namespace`.

## Visibility across namespaces

`internal` types are visible to all `.ll` files in the **same package**, regardless of namespace. `public` types export through `.sbl` under their full namespace name.

```ll
// File A
namespace Shop.Models
internal struct InternalDto { }

// File B
namespace Shop.Api
func map(d: InternalDto) { }   // OK: same package
```

## Imports

Import other **packages**, not individual files:

```ll
import Std.Json
import Std.Toon
import Shop.Models as Models
import Shop.Models.Cart      // selective import (optional)
```

Same-package types need **no import**:

```ll
namespace Shop.Api

// Shop.Models.Cart visible without import
public func createCart() -> Cart { Cart() }
```

## Name lookup

Unqualified lookup searches:

1. Local scope
2. Enclosing type
3. Current namespace
4. Parent namespaces in package
5. Imported packages (including `as` aliases)

Ambiguity is a compile error — qualify with `Models.Cart`.

## Module initialization

Package static constructors run before `main` entry (order defined by dependency graph). Per-file top-level `let` executes lazily on first access unless marked `static init`.

```ll
private let registry = loadPlugins()   // module scope; typed
```

## Root entry point

Console apps expose:

```ll
namespace App

public func main() {
    run()
}
```

Or attribute-driven entry (template-dependent):

```ll
@main
public class Program {
    public static func main() { ... }
}
```

## Splitting packages

Libraries publish `.bll` + `.sbl`; consumers never compile library source unless using a source override for debugging.

## Versioning and ABI

Public API changes follow semver:

- Patch: behavior-compatible fixes
- Minor: additive API
- Major: breaking `.sbl` changes

See packages docs for ABI stability rules.

## File organization conventions

| Convention | Guidance |
|------------|----------|
| One public type per file | Optional, not required |
| Namespace mirrors folder | Recommended |
| `internal` helpers | Same file or `_private.ll` suffix |

## Conditional compilation per platform

```ll
#if windows
import Win32.Shell
#elseif linux
import Posix.Signal
#endif
```

## Documentation and exports

`///` docs on `public` members appear in generated API reference and `.mll` context.

## Summary

| Concept | Scope |
|---------|--------|
| Package | Build, deps, versioning |
| Namespace | Name qualification |
| import | Other packages only |
| internal | Whole package |

Next: [Interop, FFI & unsafe](16-interop-ffi-unsafe.md).
