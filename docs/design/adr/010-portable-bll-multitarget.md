# ADR 010: Portable bitcode `.bll` + Go-style multi-target

**Status:** Accepted  
**Date:** 2026-10-23  
**Deciders:** LLLang core team  
**Supersedes (partially):** [ADR 003](003-prebuilt-libraries.md) native-per-triple consumer contract

## Context

ADR 003 established prebuilt `.sbl` + `.bll` so consumers never recompile dependency source. The first implementation packed **host-native object bytes** as `targets/<host_triple>.bll`. That forced publishers and registries into a per-OS matrix and blocked Go-like cross-compile from a single machine.

LLLang already lowers through LLVM. Portable bitcode can be finalized for any official triple at consumer link time, with optimization applied to the final binary.

## Decision

1. **Compiler multi-target (Go-like):** `lang build --target <alias|triple>` and optional `LANG_OS` / `LANG_ARCH`. Official MVP triples:
   - `aarch64-apple-darwin` (`darwin/arm64`, `macos/arm64`) — macOS **arm only**
   - `x86_64-unknown-linux-gnu` (`linux/amd64`)
   - `aarch64-unknown-linux-gnu` (`linux/arm64`)
   - `x86_64-pc-windows-msvc` (`windows/amd64`)
   - `aarch64-pc-windows-msvc` (`windows/arm64`)

2. **One portable `.bll` per package version:** container magic `BLL\x02` with primary payload **LLVM bitcode** (flag `PORTABLE_BITCODE`). No required native triple in the published blob.

3. **Finalize at link:** consumer `lang build --target=T` materializes bitcode → native object for `T` (cached under `.lang/cache/`), then links with the matching runtime archive under `$LANG_HOME/toolchain/lib/<triple>/`.

4. **Still no dependency source** in consumer projects. Path deps may compile from `.ll` for local library development only.

## Consequences

### Positive

- Pack once on any host; consumers build for any official target when runtime/sysroot is available.
- Final binary can be optimized for the chosen target (better than shipping frozen host objects).
- Lockfile and store carry a **single** `.bll` hash.

### Negative

- First build for a new target pays materialization cost (mitigated by `.lang/cache`).
- Cross-link still needs clang + runtime (and often a sysroot) for non-host triples.
- Old native `BLL\x01` / raw-object blobs must be republished (`lang pack`).

### Non-goals

- Fat universal executables.
- Shipping dependency `.ll` source via the registry.
- macOS x86_64 as an official target.

## References

- [`.bll` format](../../formats/bll.md)
- [Package manager](../../packages/package-manager.md)
- [ADR 003](003-prebuilt-libraries.md)
