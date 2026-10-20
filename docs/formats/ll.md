# `.ll` source format

**LLLang source** uses the `.ll` extension. Files are UTF-8 text parsed by `lllang-syntax`, type-checked by the compiler (Phase 2+), and lowered to LLVM IR for native code. This document describes the role of `.ll` files in the ecosystem — not the full language (see the [language guide](../language/01-lexical-structure.md)).

## Purpose

| Concern | Where it lives |
|---------|----------------|
| Human-authored program logic | `.ll` |
| Published API surface for consumers | `.sbl` (generated or hand-curated) |
| Native code + linkable objects | `.bll` (per target) |
| LLM / agent context | `.mll` (TOON) |

Application and library **authors** edit `.ll`. Dependency **consumers** normally never see dependency `.ll` — only `.sbl` and `.bll`.

## File identity

- One primary **module** per file is typical; large packages split by namespace across files.
- The **package** name comes from `lang.toml` (`[package].name`), not from the filename.
- **Namespaces** (`namespace Acme.Billing`) organize declarations and affect name lookup in `import`.

## What goes in source

`.ll` files contain:

- `import` directives
- Types: `struct`, `class`, `enum`, `protocol`, type aliases
- Functions, methods, properties, operators
- Attributes (`@Test`, `@Main`, `@AbiExport`, …)
- Top-level statements only where the grammar allows (e.g. script-style entry in small examples)

Example module fragment:

```ll
namespace Demo.Greeter

import Std.IO

public struct Config(greeting: String)

public func run(config: Config) {
    print(config.greeting)
}
```

## Compilation unit graph

The toolchain builds a **module DAG**:

1. Parse all reachable `.ll` files (roots: `[package].entry`, library `public-api` globs).
2. Resolve `import` to other modules in the same package or to `.sbl` stubs for dependencies.
3. Typecheck and monomorphize generics (Phase 2+).
4. Emit object code into the output binary or into a `.bll` when packing a library.

`lang check` (Phase 1) performs step 1 only and emits parse diagnostics.

## Encoding and line endings

- UTF-8; BOM ignored.
- `\n`, `\r\n`, or `\r` newlines — normalized internally.
- No required semicolons; see [lexical structure](../language/01-lexical-structure.md).

## Relationship to `.sbl`

When you **publish** a library, the toolchain extracts the **public API** from `.ll` (respecting `public` / `internal` and export globs) into `.sbl`. The `.sbl` file must **ABI-match** the bundled `.bll` (hash recorded in both). Consumers typecheck against `.sbl` and link `.bll`.

You may check in `.sbl` for review in rare cases, but the registry canonical pair is `(sbl, bll)`.

## Relationship to `.mll`

`lang llm gen` scans `.ll` (and public `.sbl`) to emit `.mll` TOON summaries for agents. Source remains the truth; `.mll` is derived metadata.

## Tooling touchpoints

| Tool | Uses `.ll` |
|------|------------|
| `lang check` | Parse all inputs |
| `lang fmt` | Rewrite formatted source (Phase 2+) |
| `lang lsp` | Diagnostics, symbols, folding, semantic tokens |
| `lang doc` | Extract `///` documentation |
| `lang test` / `bench` | Compile test/bench modules |
| Editor TextMate / Tree-sitter | Syntax highlight |

## Versioning

Language **syntax** evolves with compiler semver. The `.ll` grammar is specified in [grammar.ebnf](../spec/grammar.ebnf). Breaking syntax changes bump compiler major version and are called out in release notes; `.bll` ABI is versioned separately (see [Versioning & ABI](../packages/versioning-and-abi.md)).

## See also

- [`.bll` binary library](bll.md)
- [`.sbl` declarations](sbl.md)
- [`.mll` metadata](mll.md)
- [Build pipeline](../toolchain/build-pipeline.md)
