# `.bll` binary library format

A **`.bll` (binary LLLang library)** is a **portable** prebuilt artifact produced by `lang pack`. Consumers **finalize** it for a chosen `--target` at link time; they **never recompile** dependency `.ll` source.

See [ADR 010](../design/adr/010-portable-bll-multitarget.md).

## Design principles

1. **One `.bll` per package version** — not one file per OS. The published blob is target-independent LLVM bitcode.
2. **Immutable at rest** — content-addressed in `~/.lang/store`; registries store one blob by hash.
3. **Paired with `.sbl`** — types and `@abi` contracts live in `.sbl`; code lives in `.bll`. A hash ties the pair together.
4. **Finalize, don’t recompile source** — the consumer toolchain lowers bitcode → native object for the build triple (with optimization), then links. Dependency `.ll` is not in the graph.

## On-disk layout (BLL v2)

A `.bll` file is a little-endian length-prefixed container:

```text
┌─────────────────────────────────────────────────────────────┐
│ HEADER (magic BLL\x02, version, ABI epoch, PORTABLE flag)   │
├─────────────────────────────────────────────────────────────┤
│ PACKAGE ID (name + semver)                                  │
├─────────────────────────────────────────────────────────────┤
│ LLVM BITCODE (primary payload)                              │
├─────────────────────────────────────────────────────────────┤
│ ABI SYMBOL TABLE (optional)                                 │
├─────────────────────────────────────────────────────────────┤
│ RESOURCES (optional)                                        │
├─────────────────────────────────────────────────────────────┤
│ HASH (integrity + .sbl binding)                             │
└─────────────────────────────────────────────────────────────┘
```

### Header

| Field | Description |
|-------|-------------|
| Magic | `BLL\x02` |
| Version | Container format version (`2`) |
| ABI version | Language ABI epoch; must match consumer compiler |
| Flags | `PORTABLE_BITCODE` required |
| Package id | Name + semver of producing package |

Native target triple is **not** stored in the published header. Materialized objects for a triple live only under the consumer’s `.lang/cache/`.

### LLVM bitcode

Primary payload: LLVM bitcode for the library’s compiled MIR (monomorphized public API). The publisher does **not** ship host `.o` / `.obj`.

On `lang build --target=T`, the toolchain:

1. Parses the bitcode section
2. Sets module triple/data layout for `T`
3. Emits a native object for `T`
4. Links with `$LANG_HOME/toolchain/lib/<T>/liblllang_rt.a` (or Windows equivalent)

### Migration from v1

Artifacts that are raw native objects or magic `BLL\x01` are **rejected** with a diagnostic to republish via `lang pack` (BLL v2). There is no automatic conversion.

## Producer workflow

```bash
cd my-lib
lang pack
# → .lang/dist/package.sbl + package.bll (portable bitcode)
```

`lang pack` is independent of the host OS for the published blob shape.

## Consumer workflow

```bash
lang build --target linux/amd64
# or: LANG_OS=linux LANG_ARCH=amd64 lang build
```

1. Resolver loads `.sbl` + portable `.bll` from the store.
2. Compiler typechecks against `.sbl` only.
3. Linker materializes `.bll` → object for the active triple (cached), then links.

## Official targets (MVP)

| Alias | Triple |
|-------|--------|
| `darwin/arm64`, `macos/arm64` | `aarch64-apple-darwin` |
| `linux/amd64` | `x86_64-unknown-linux-gnu` |
| `linux/arm64` | `aarch64-unknown-linux-gnu` |
| `windows/amd64` | `x86_64-pc-windows-msvc` |
| `windows/arm64` | `aarch64-pc-windows-msvc` |

macOS x86_64 is **not** an official target.

## What `.bll` is not

- Not a fat multi-slice native archive
- Not a tree of dependency source
- Not recompilable from `.ll` by end users — fork and republish if you need different public API codegen

## Tooling

| Command | Interaction |
|---------|-------------|
| `lang pack` | Emit portable `.bll` + `.sbl` |
| `lang publish` | Upload one `.bll` blob |
| `lang restore` | Verify single `bll` hash |
| `lang build --target …` | Finalize bitcode for triple |
| `lang --info` | List supported BLL version + official triples |

## See also

- [`.sbl` type declarations](sbl.md)
- [Versioning & ABI](../packages/versioning-and-abi.md)
- [Package manager](../packages/package-manager.md)
