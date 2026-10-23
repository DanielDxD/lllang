# ADR 003: Prebuilt libraries (`.bll` / `.sbl`)

**Status:** Accepted  
**Date:** 2026-03-20  
**Deciders:** LLLang core team

## Context

Source-based package managers (npm, Cargo path deps everywhere, Swift PM resolving source) force consumers to recompile transitive dependencies. That increases CI time, flakes on compiler version drift, and expands the attack surface of build scripts.

LLLang aims at **fast, reproducible application builds** and binary distribution of libraries per target triple. Consumers need typed APIs without importing vendor source trees into their repositories.

## Decision

Published dependencies ship as:

| Artifact | Contents |
|----------|----------|
| **`.bll`** | Portable LLVM bitcode library (BLL v2); finalized to native code for `--target` at link (see [ADR 010](010-portable-bll-multitarget.md)) |
| **`.sbl`** | Textual stub declarations: public types, functions, attributes, version, ABI hash |

The package manager (`lang add`, lockfile) resolves **versions**, fetches one `.bll` + `.sbl`, and **materializes** `.bll` for the active build target during `lang build`. Source for third-party libraries is **not** copied into consumer projects by default.

Authors publish via `lang pack` (Phase 3), uploading artifacts to a registry. Local path overrides may point at source for library development only.

Lockfile (`Package.lock`) records exact package versions, content hashes, and target matrix. Package metadata lives in evaluated **`Package.ll`** manifests (not TOML).

## Consequences

### Positive

- Application CI links instead of compiling hundreds of modules.
- ABI checks (`LL4001`) catch mismatches between compiler and `.bll`.
- Smaller checkouts for apps and agents cloning repos.
- Clear boundary for security review (binary + stub vs arbitrary build scripts).

### Negative

- Library authors publish once (portable bitcode); consumers need a runtime/sysroot for each cross-link target.
- Debugging into dependency requires separate symbol/debug package or source symbol server (future).
- Faster iteration during library dev requires path dependency workflow documented in package guide.

### Non-goals (reaffirmed)

- No `node_modules`-style hoisted source trees in apps.
- No mandatory recompile-from-source for registry packages.

## References

- [`.bll` format](../../formats/bll.md)
- [`.sbl` format](../../formats/sbl.md)
- [Package manager](../../packages/package-manager.md)
- [Diagnostics LL5xxx](../../spec/diagnostics.md)
