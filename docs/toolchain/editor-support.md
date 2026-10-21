# Editor support

LLLang targets a consistent editing experience in **VS Code / Cursor**, on the **web** (Shiki, highlight.js), and eventually other IDEs. Phase 1 delivers TextMate grammar, basic semantic tokens from the LSP, and extension wiring for `lang lsp`.

## VS Code / Cursor extension

Location in repository: `editors/vscode`.

### Features

| Feature | Mechanism |
|---------|-----------|
| Syntax highlighting | TextMate grammar (`syntaxes/lllang.tmLanguage.json`) |
| Bracket matching | Language configuration |
| Comment toggling | `//` and `/* */` |
| LSP diagnostics, symbols, folding | `lang lsp` |
| Snippets | `snippets/lllang.json` (optional) |

### Installation

- **Marketplace:** search “LLLang” (when published)
- **Development:** `Extensions: Install from VSIX` or open `editors/vscode` in Extension Development Host

### Settings

| Setting | Default | Purpose |
|---------|---------|---------|
| `lllang.langPath` | `lang` | Path to CLI binary |
| `lllang.trace.server` | `off` | LSP log verbosity |

Example:

```json
{
  "lllang.langPath": "/usr/local/bin/lang"
}
```

### TextMate grammar

Scopes follow predictable patterns for theming:

| Scope | Examples |
|-------|----------|
| `keyword.control.lllang` | `if`, `switch`, `throws` |
| `storage.type.lllang` | `struct`, `class`, `protocol` |
| `entity.name.type.lllang` | Type identifiers |
| `entity.name.function.lllang` | Functions |
| `comment.line.documentation.lllang` | `///` docs |

Grammar stays aligned with [lexical structure](../language/01-lexical-structure.md) and [keywords](../spec/keywords.md).

## Web highlighting

Location: `editors/web`.

### Shiki

Bundled grammar compatible with Shiki’s VS Code theme engine:

```ts
import { createHighlighter } from 'shiki'

const highlighter = await createHighlighter({
  langs: ['lllang'],
  themes: ['github-dark']
})
```

Ship `lllang.tmLanguage.json` or a converted Shiki JSON grammar.

### highlight.js

Register a minimal `highlight.js` language module for docs sites:

```js
hljs.registerLanguage('ll', lllangHighlight)
```

Supports keywords, strings, numbers, and nested comments — sufficient for blogs; TextMate remains authoritative for edge cases.

## Other editors

| Editor | Status |
|--------|--------|
| Neovim | Experimental stub: [`editors/tree-sitter-lllang`](../../editors/tree-sitter-lllang) (TextMate remains primary) |
| JetBrains | Planned |
| Zed | TextMate import |

Until dedicated plugins exist, use TextMate grammar + LSP client templates (nvim-lspconfig pattern).

## Formatting and linting in editor

- **Format on save:** requires Phase 2 LSP `documentFormatting` → `lang fmt`
- **Lint:** parse diagnostics Phase 1; type diagnostics Phase 2

## Recommended extensions (users)

None required beyond LLLang. Optional: TOON/JSON viewers for `.mll` sidecars.

## Contributing grammar changes

1. Update `editors/vscode/syntaxes/lllang.tmLanguage.json`
2. Sync copy to `editors/web`
3. Add fixture files under `editors/vscode/test/` (colorized snapshots)
4. Document new tokens in [keywords](../spec/keywords.md) if reserved words change

## See also

- [LSP](lsp.md)
- [Installation](../getting-started/installation.md)
- [CLI `lsp`](cli-reference.md)
