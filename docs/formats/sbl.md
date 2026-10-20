# `.sbl` type declaration format

An **`.sbl` (surface binary library)** file is a **textual**, human-readable declaration of a published library's public API and ABI. It contains no executable code. The compiler uses `.sbl` for parsing, name resolution, and type checking when consuming a dependency; the linker satisfies symbols from the matching **`.bll`**.

## Role in the dependency model

```text
Consumer compile time          Link time
─────────────────────         ─────────
import Acme.Widgets    →   types from package.sbl
                           symbols from targets/<triple>.bll
```

**.sbl first, then .bll** — the package manager always materializes `.sbl` before selecting a target `.bll`. If hashes disagree, restore fails.

## File properties

- Encoding: UTF-8
- Line endings: normalized to `\n` in canonical form for hashing
- Syntax: subset of `.ll` declaration syntax + ABI annotations
- No function bodies except `=>` expression members allowed in protocols/properties when needed for witness clarity (bodies may be omitted with `{ ... }` stubs)

## Contents

A typical `.sbl` includes:

1. **Package header** — name, version, abi version
2. **Imports** — re-exported dependencies (by package id, not path)
3. **Type declarations** — `struct`, `class`, `enum`, `protocol`, aliases
4. **Members** — signatures only: methods, properties, operators
5. **`@abi` attributes** — stable symbol names for linkers
6. **Integrity footer** — comment block with `.bll` content hash (optional in hand-edited files; required after `lang pack`)

## Example

Below is a trimmed `.sbl` for a fictional `acme.math` library. The `@abi` strings must match rows in the paired `.bll` symbol table.

```ll
// sbl-format: 1
package acme.math@1.4.0
abi 2

namespace Acme.Math

@abi("acme_math_vec2_init")
public struct Vec2(public let x: Float64, public let y: Float64): Equatable {
    @abi("acme_math_vec2_add")
    public func adding(other: Vec2) -> Vec2
}

@abi("acme_math_distance")
public func distance(a: Vec2, b: Vec2) -> Float64

// bll-hash: blake3:4f8c2e1a9b0d3c7e8f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f7
// target: aarch64-apple-darwin
```

When the same library ships multiple targets, **one `.sbl` is shared**; each `.bll` repeats the **same** `bll-hash` binding in the footer comment or the pack tool verifies a single sbl-hash field in manifest metadata.

## `@abi` attribute

`@abi("symbol_name")` marks declarations that participate in the **stable ABI**:

| Applies to | Purpose |
|------------|---------|
| Types with exported ctors | Factory / metadata symbols |
| Functions / methods | Link name for native code |
| Static properties | Getter/setter symbols |

Rules:

- ABI names are globally unique per **package + major ABI epoch** (collisions across packages are OK at link layer via mangling prefix derived from package id).
- Changing an `@abi` string or signature without bumping ABI version is forbidden.
- Removing an `@abi` export is a breaking change.

Internal helpers omitted from `.sbl` are not linkable from consumers.

## Canonical hashing

`lang pack` computes:

1. **SBL canonical text** — strip comments except hash footer, normalize whitespace, sort re-exports deterministically
2. **SBL digest** — BLAKE3 of canonical text
3. **BLL digest** — BLAKE3 of entire `.bll` file
4. Cross-check: `.bll` HASH section stores SBL digest; `.sbl` footer stores BLL digest

Example matching pair:

| Artifact | Field |
|----------|-------|
| `package.sbl` footer | `bll-hash: blake3:4f8c…` |
| `aarch64-apple-darwin.bll` HASH section | `sbl-hash: blake3:a91e…` |

`lang restore` verifies both directions.

## Visibility

Only **`public`** API appears in published `.sbl`. Optional `publicHeaders` / export filters in `Package.ll` may further restrict exports. `internal` and `fileprivate` declarations never ship.

## Generated vs hand-maintained

- **Default:** `lang pack` generates `.sbl` from `.ll`.
- **Advanced:** teams may post-process for doc comments or compatibility shims; pack step re-validates against compiled symbols.

Hand editing cannot introduce types that lack `.bll` backing.

## Consumer compiler behavior

1. Parse `.sbl` as a external module with `external` linkage.
2. Typecheck calls against signatures.
3. Emit imports to linker using `@abi` symbol names.
4. Refuse to inline bodies not present in `.sbl` unless LTO bitcode section explicitly allows (see [`.bll`](bll.md)).

## Version skew diagnostics

| Error | Meaning |
|-------|---------|
| `E_ABI_MISMATCH` | Compiler ABI epoch ≠ sbl `abi` field |
| `E_SBL_BLL_HASH` | Footer hash does not match store |
| `E_MISSING_SYMBOL` | `@abi` entry missing in `.bll` table |

Fix by `lang update` / pinning in `Package.lock`.

## See also

- [`.bll` binary layout](bll.md)
- [Versioning & ABI](../packages/versioning-and-abi.md)
- [Package manager](../packages/package-manager.md)
