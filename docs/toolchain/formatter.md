# Formatter (`lang fmt`)

The formatter rewrites `.ll` source to a **canonical style** so diffs stay focused on semantics. It is implemented as a pretty-printer over the syntax tree (Phase 2+), sharing the same grammar as `lang check`.

## Usage

```bash
lang fmt                  # format all .ll in package
lang fmt src/foo.ll
lang fmt --check          # exit non-zero if any file would change
lang fmt --write          # default; explicit for scripts
```

Workspace:

```bash
lang fmt --workspace
```

## Design goals

1. **Idempotent** — running twice produces identical output
2. **Non-destructive semantics** — only whitespace, optional parentheses, and line breaks change
3. **Comment preservation** — `//`, `/* */`, and `///` doc comments stay attached to declarations
4. **Deterministic** — no column-width randomness across platforms

## Style rules (summary)

| Construct | Rule |
|-----------|------|
| Indent | 4 spaces per level |
| Line width | Soft wrap at 100 columns; break before operators |
| Braces | Same line as header for types; Allman for long `switch` arms (configurable later) |
| Imports | Sorted alphabetically, one per line |
| Blank lines | One between top-level declarations; two before `namespace` sections optional |
| Semicolons | Omitted when newline suffices |
| Trailing commas | Allowed and preserved in multiline lists |

Example before/after:

```ll
// before
let x=1+2

// after
let x = 1 + 2
```

```ll
// before
public func foo(a:Int,b:String)->Bool{return a>0&&b.count>0}

// after
public func foo(a: Int, b: String) -> Bool {
    return a > 0 && b.count > 0
}
```

## Scope

- Formats `.ll` only — not `.sbl`, `.mll`, or `lang.toml`
- Generated `.sbl` from `lang pack` uses its own canonicalizer for hashes

## Editor integration

Format on save calls `lang fmt` with file path (Phase 2 LSP `textDocument/formatting`). Until then, use CLI in a file watcher or pre-commit hook.

## CI

```bash
lang fmt --check
```

Fail the job if developers forgot to format.

## Configuration (planned)

`lang.toml`:

```toml
[fmt]
max-width = 100
indent = 4
```

Until implemented, defaults above apply.

## See also

- [CLI reference](cli-reference.md)
- [Lexical structure](../language/01-lexical-structure.md)
