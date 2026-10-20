# `.bll` binary library format

A **`.bll` (binary LLLang library)** file is a self-contained, target-specific native artifact produced when a library package is built for publishing or local consumption. Consumers **link** `.bll` files; they **never recompile** dependency source to use a published library.

## Design principles

1. **One `.bll` per target triple** — e.g. `aarch64-apple-darwin.bll`, `x86_64-unknown-linux-gnu.bll`. Cross-compilation produces one file per requested target.
2. **Immutable at rest** — content-addressed in `~/.lang/store`; registries store blobs by hash.
3. **Paired with `.sbl`** — types and `@abi` contracts live in `.sbl`; native code and symbols live in `.bll`. A cryptographic hash ties the pair together.
4. **No consumer codegen for deps** — generic instantiation inside a dependency is resolved when the **library** was built; remaining generic/inline material is embedded as LLVM bitcode sections (see below).

## On-disk layout

A `.bll` file is a length-prefixed binary container (little-endian). Sections appear **in order**:

```text
┌─────────────────────────────────────────────────────────────┐
│ HEADER                                                      │
├─────────────────────────────────────────────────────────────┤
│ NATIVE OBJECTS (platform object files / merged .o blobs)    │
├─────────────────────────────────────────────────────────────┤
│ LLVM BITCODE (generics / forced inline templates)           │
├─────────────────────────────────────────────────────────────┤
│ ABI SYMBOL TABLE                                            │
├─────────────────────────────────────────────────────────────┤
│ RESOURCES (embedded assets, strings, reflection tables)     │
├─────────────────────────────────────────────────────────────┤
│ HASH (integrity + .sbl binding)                             │
└─────────────────────────────────────────────────────────────┘
```

### Header

| Field | Description |
|-------|-------------|
| Magic | `BLL\x01` |
| Version | Container format version (u16) |
| ABI version | Language ABI epoch (u16); must match consumer compiler |
| Target triple | NUL-terminated string (`aarch64-apple-darwin`, …) |
| Package id | Name + semver of producing package |
| Section count | Usually 5 payload sections + hash |
| Flags | e.g. `PIC`, `LTO`, `UNWIND_TABLES` |

The header allows tools to reject wrong-target or wrong-ABI artifacts before parsing bodies.

### Native objects

Contains one or more **native object files** (ELF `.o`, Mach-O `.o`, COFF `.obj`) or a single merged object blob per platform convention. This is what the linker consumes for:

- Non-generic functions and methods
- Statically monomorphized instantiations performed at library build time
- Runtime support objects (ARC helpers, protocol witnesses emitted as concrete)

Objects are stored with alignment suitable for mmap; optional compression (zstd) is indicated in header flags.

### LLVM bitcode (generics / inline)

When the library exposes **generic** APIs or symbols marked for cross-crate inlining, the producer may embed **LLVM bitcode** modules for:

- Templates not monomorphized at library build time (consumer monomorphization is **forbidden** for published libs — the library builder must emit all supported specializations or provide bitcode only for whitelisted `@InlineGeneric` APIs)
- `@ForceInline` bodies referenced across package boundaries

The consumer linker uses this section only through the **toolchain's blessed LTO path** when building an executable that links the `.bll`; consumers do not run `llc` on dependency `.ll` source.

Phase 2 policy: default publish mode **monomorphizes all public generics** into native objects; bitcode section is optional and feature-gated for advanced interop.

### ABI symbol table

Maps stable **ABI names** (see `@abi` in `.sbl`) to:

- Symbol name in native objects
- Signature encoding (calling convention, struct return, sret)
- Generic instantiation key (if applicable)
- Weak / optional linkage flags

The loader verifies that every `@abi` entry in the paired `.sbl` resolves to a symbol row. Missing symbols fail at link time with a diagnostic referencing package version and hash.

### Resources

Optional embedded data:

- String pools for reflection / errors
- Codable schema tables
- Static assets declared in `Package.ll` target `resources:` entries
- Debug info bundles (split DWARF may live beside `.bll` in debug builds)

Resources are read-only at runtime; mutable state stays in normal `.ll` static initialization linked from native objects.

### Hash

Final section (also duplicated as footer checksum):

- **BLAKE3** digest over all preceding sections
- **SBL binding hash** — digest of the canonical `.sbl` ABI text
- **Source revision** — optional git commit or `lang pack` build id

`lang restore` and the linker compare `.sbl` on disk with the hash embedded in `.bll`. Mismatch is a hard error — never silently relink.

## Producer workflow

```bash
cd my-lib
lang build --release --target aarch64-apple-darwin
lang pack --target aarch64-apple-darwin
# → produces package.sbl + targets/aarch64-apple-darwin.bll
```

`lang pack` validates:

- Public API ⊆ compiled symbols
- ABI table complete
- Hash written and self-consistent

## Consumer workflow

When app `App` depends on `acme.widgets@1.2.3`:

1. Resolver fetches `.sbl` + `.bll` for active target into the store.
2. Compiler loads types from `.sbl` only.
3. Linker loads native objects + symbol table from `.bll`.
4. No `.ll` from `acme.widgets` is required or allowed in the consumer graph.

```bash
lang build   # selects .bll matching host or --target
```

## What `.bll` is not

- Not a fat archive of source files
- Not a JVM JAR or .NET assembly (though conceptually similar to a native `.lib` / `.a` with metadata)
- Not recompilable by end users — fork the library and publish a new package if you need different optimizations

## Tooling

| Command | Interaction |
|---------|-------------|
| `lang pack` | Create `.bll` + update hashes |
| `lang publish` | Upload blob to registry |
| `lang restore` | Verify hashes after download |
| `lang --info` | List supported BLL container version |

## See also

- [`.sbl` type declarations](sbl.md)
- [Versioning & ABI](../packages/versioning-and-abi.md)
- [Package manager](../packages/package-manager.md)
- [Build pipeline](../toolchain/build-pipeline.md)
