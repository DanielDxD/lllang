# Language Server (`lang lsp`)

The **`lang lsp`** subcommand runs an LSP server over **stdio** (JSON-RPC). Editors spawn it via their LSP client configuration. Implementation lives in `lllang-lsp` on top of `lllang-syntax` and grows with semantic analysis in Phase 2+.

## Starting the server

```bash
lang lsp
```

Editor config (VS Code / Cursor) typically sets:

```json
{
  "command": "lang",
  "args": ["lsp"]
}
```

The server runs until stdin closes.

## Phase 1 capabilities

Available in the current roadmap tranche:

| LSP feature | Support | Notes |
|-------------|---------|-------|
| `textDocument/publishDiagnostics` | Yes | Parse errors only |
| `textDocument/documentSymbol` | Yes | Flat/outline symbols from AST |
| `textDocument/foldingRange` | Yes | Blocks, collections, comments |
| `textDocument/semanticTokens` | Partial | Syntax-level token kinds |
| `initialize` | Yes | Advertises Phase 1 subset |
| Document sync | Yes | Incremental |

**Not in Phase 1:**

- Go to definition / type definition
- Find references
- Rename
- Completion (beyond trivial keyword list)
- Hover types
- Code actions / fixes
- Inlay hints
- Formatting (`lang fmt` not wired)

### Phase 1 `initialize` advertisement (conceptual)

```json
{
  "capabilities": {
    "textDocumentSync": { "change": 2 },
    "documentSymbolProvider": true,
    "foldingRangeProvider": true,
    "semanticTokensProvider": {
      "legend": { "tokenTypes": ["namespace", "type", "function", "keyword", ...] }
    }
  }
}
```

Exact legend matches TextMate scopes where possible for consistent theming.

## Phase 2 capabilities (planned)

Once name resolution and types exist:

| Feature | Description |
|---------|-------------|
| `definition` / `typeDefinition` | Jump to symbol across modules |
| `references` | Include `.sbl` imported symbols (read-only) |
| `hover` | Types + `///` docs |
| `completion` | Contextual members and imports |
| `documentFormatting` | Delegate to `lang fmt` |
| `rename` | Workspace edit with conflict checks |
| `inlayHints` | Parameter names, inferred types (local only) |
| `codeAction` | Quick fixes for parse and simple semantic issues |

Diagnostics expand to type errors, unused bindings, and ABI export gaps in libraries.

## Phase 4 / 1.1d capabilities (MVP shipped)

- Type-aware completion: symbols from sema index; members after `.` from HIR + builtins **and extension methods**
- Hover: type + `throws` + `#deprecated` when indexed
- Code actions: LL2005 insert `try`; LL2000 add `override`; LL2002 `as` → `as?`/`as!`; LL3001 empty array → annotate `[Int]`
- Still Later: protocol-constraint completion, code lenses, pull diagnostics, `llm/explain`

## Multi-root workspaces

Monorepos with `Solution.ll` and member `Package.ll` files:

- Opening the solution root enables multi-package diagnostics (planned)
- One LSP instance per editor window root (default)
- Future: workspace folders with per-folder `Package.ll` detection

## Performance

- Syntax parse cached per URI with content hash
- Incremental parse reuses unaffected subtrees (planned optimization)
- Semantic analysis scoped to dirty files + importers

## Debugging

```bash
LANG_LOG=debug lang lsp 2>/tmp/lang-lsp.log
```

Editor trace: enable LSP message logging in VS Code (`LLLang trace server`).

## See also

- [Editor support](editor-support.md)
- [CLI reference](cli-reference.md)
- [Installation](../getting-started/installation.md)
