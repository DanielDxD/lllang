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
| **`.bll`** | Precompiled native library for a specific target (object/archive with LLLang ABI) |
| **`.sbl`** | Textual stub declarations: public types, functions, attributes, version, ABI hash |

The package manager (`lang add`, lockfile) resolves **versions and targets**, fetches `.bll` + `.sbl`, and links `.bll` during `lang build`. Source for third-party libraries is **not** copied into consumer projects by default.

Authors publish via `lang pack` (Phase 3), uploading artifacts to a registry. Local path overrides may point at source for library development only.

Lockfile (`Package.lock`) records exact package versions, content hashes, and target matrix. Package metadata lives in evaluated **`Package.ll`** manifests (not TOML).

## Consequences

### Positive

- Application CI links instead of compiling hundreds of modules.
- ABI checks (`LL4001`) catch mismatches between compiler and `.bll`.
- Smaller checkouts for apps and agents cloning repos.
- Clear boundary for security review (binary + stub vs arbitrary build scripts).

### Negative

- Library authors must build matrix (macOS arm64/x64, Linux, Windows) or use CI to publish.
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
