# Build pipeline

The LLLang build pipeline turns `.ll` sources and prebuilt `.bll` dependencies into executables or new library artifacts. Phase 1 implements parsing only; this document describes the **target architecture** aligned with [ROADMAP](../../ROADMAP.md).

## High-level flow

```text
lang.toml + lang.lock
        │
        ▼
┌───────────────┐     ┌─────────────┐     ┌──────────────┐
│ Resolve deps  │────▶│ Load .sbl   │     │ Select .bll  │
│ (store/index) │     │ (types)     │     │ (target)     │
└───────────────┘     └─────────────┘     └──────────────┘
        │
        ▼
┌───────────────┐     ┌─────────────┐     ┌──────────────┐
│ Parse .ll     │────▶│ Semantic    │────▶│ LLVM IR      │
│ (lllang-syntax)│     │ analysis    │     │ (inkwell)    │
└───────────────┘     └─────────────┘     └──────────────┘
        │                                        │
        ▼                                        ▼
┌───────────────┐                         ┌──────────────┐
│ Diagnostics   │                         │ Native link  │
└───────────────┘                         │ + .bll objs  │
                                          └──────────────┘
                                                │
                                                ▼
                                          executable or .bll
```

## Stages

### 1. Manifest and graph resolution

- Read `[package]`, `[dependencies]`, `[workspace]`
- Load `lang.lock`; fetch missing packages into `~/.lang/store`
- Build DAG of package references; reject cycles
- Choose **host triple** or `--target` from `[build]` / CLI

### 2. External interface loading

For each dependency:

1. Parse `package.sbl` into the external module table
2. Map `@abi` symbols for later link
3. mmap target-specific `.bll` (native objects + symbol table)

Consumers never parse dependency `.ll`.

### 3. Frontend (Phase 1 partial)

| Step | Crate / component | Output |
|------|-------------------|--------|
| Lex + parse | `lllang-syntax` | AST + parse errors |
| Module merge | compiler | Single package AST forest |

`lang check` stops here.

### 4. Semantic analysis (Phase 2)

- Name binding across namespaces and imports
- Type checking with local inference only
- Protocol witness tables
- ARC ownership / borrow checking for classes and closures
- Generic monomorphization plan for **current** package
- `@abi` export validation for libraries

### 5. Middle-end and LLVM

- Lower AST to LLVM IR via **inkwell** (LLVM 22)
- Apply ARC retain/release insertion
- Optional LTO with embedded bitcode from dependency `.bll` (policy-limited)
- Optimization passes: `-O0` debug, `-O2`/`-O3` release

### 6. Codegen and linking

- Emit object files to `.lang/build/obj/`
- Invoke platform linker (ld, lld, link.exe) with:
  - Objects from current package
  - Native object sections from dependency `.bll` files
  - System libraries from `[build].link`
- Apps → `.lang/build/bin/<name>`
- Libs → `targets/<triple>.bll` via [pack](cli-reference.md#lang-pack)

### 7. Incremental builds (planned)

- File-level AST hashes
- IR cache keyed by symbol + dependency `.bll` hashes
- `lang watch` reuses incremental state

## Profiles

| Profile | Flags | Use |
|---------|-------|-----|
| Debug | default | Fast compile, symbols |
| Release | `--release` | Optimized, stripped optional |
| CI | `LANG_CI=1` | Deterministic, warnings as errors |

## Cross-compilation

```bash
lang sdk install aarch64-unknown-linux-gnu
lang build --target aarch64-unknown-linux-gnu
```

SDK provides LLVM tools + sysroot; each target produces its own `.bll` when packing libraries.

## Diagnostics pipeline

Errors carry:

- Stable codes (`E0309`, …) — see [diagnostics](../spec/diagnostics.md)
- Primary span + related spans
- Notes for import chains across packages

Parse errors (Phase 1) omit type-related codes.

## Library publish path

```bash
lang build --release --target <triple>
lang pack
```

Pack verifies `.sbl`/`.bll` hash pairing before tarball creation.

## Failure modes

| Failure | Stage |
|---------|-------|
| Lockfile / hash mismatch | Resolve |
| Missing `@abi` symbol | Link |
| ABI version mismatch | Load `.sbl` / `.bll` |
| Type error | Semantic |
| LLVM verify error | Codegen |

## See also

- [`.bll` format](../formats/bll.md)
- [Manifest](manifest.md)
- [CLI `build`](cli-reference.md)
