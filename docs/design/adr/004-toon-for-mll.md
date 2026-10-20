# ADR 004: TOON for `.mll` metadata

**Status:** Accepted  
**Date:** 2026-03-20  
**Deciders:** LLLang core team

## Context

LLM-assisted development needs compact, structured context: module purpose, public APIs, invariants, and example call patterns. JSON is universal but verbose (quotes, braces, repeated keys). YAML is human-friendly but ambiguous for strict parsers.

**TOON (Token-Oriented Object Notation)** is designed as a line-oriented, indentation-scoped format with explicit types and minimal punctuation — easier for models to emit and parse than JSON while remaining machine-readable.

LLLang introduces **`.mll` files** as LLM-oriented metadata attached to packages or modules, consumed by `lang llm gen` and `Std.Llm` tooling.

## Decision

Standardize **`.mll` encodings on TOON** as the canonical on-disk format. JSON remains supported for interchange via `Std.Json` and round-trip conversion in `Std.Toon`, but generators and official templates emit TOON by default.

`.mll` documents include:

- Module identity (`namespace`, version, targets)
- Public API summaries (signatures, doc strings)
- Constraints (`requires`, `throws`, thread safety)
- Optional embedded examples and tool schemas for agents

The CLI command `lang llm gen` walks the parse tree (and later sema) to emit `.mll` next to sources or aggregated at package root.

## Consequences

### Positive

- Lower token footprint in agent context windows vs JSON.
- Single format aligned with `Std.Toon` in the standard library story.
- Human skim-friendly in code review (line-based records).

### Negative

- Ecosystem must learn TOON; mitigated by JSON export and docs in `lllang-json-toon` skill.
- Editor highlighting for TOON is separate from `.ll` (planned TextMate scope).
- Strict parsers must reject ambiguous indentation (similar to YAML footguns); TOON spec in stdlib docs will define error recovery.

### Follow-ups

- Define TOON schema for `.mll` v1 in [formats/mll.md](../../formats/mll.md).
- Wire `lang llm gen` to syntax-only exports in Phase 1, semantic enrichment in Phase 3.

## References

- [LLM overview](../../llm/overview.md)
- [Philosophy](../philosophy.md)
- Skill: `lllang-json-toon`
