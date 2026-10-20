# LLLang for VS Code / Cursor

Syntax highlighting, snippets, and LSP integration for LLLang (`.ll`), signature/binary listings (`.sbl`), and manifest docs (`.mll`).

## Requirements

- VS Code **1.85+** or Cursor with compatible extension host
- [LLLang CLI](https://github.com/lllang/lllang) (`lang`) on your `PATH` for language-server features (completion, diagnostics, semantic tokens)

## Install from source

```bash
cd editors/vscode
npm install
npm run compile   # optional — out/extension.js is prebuilt
```

Then open this folder in VS Code and press **F5** (Run Extension), or package a VSIX:

```bash
npm run package
code --install-extension lllang-0.1.0.vsix
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `lllang.serverPath` | `lang` | Executable used to run `lang lsp` |

Example `settings.json`:

```json
{
  "lllang.serverPath": "/usr/local/bin/lang"
}
```

## Features

- TextMate grammar for `.ll` (primary constructors, string interpolation, async/await, etc.)
- `.sbl` reuses the LLLang grammar with SBL header directives
- `.mll` TOON-style manifest highlighting
- Snippets: `class`, `struct`, `enum`, `func`, `protocol`, `switch`, `guard`, `throws`
- LSP via `lang lsp` for `.ll` and `.sbl` files

## Development

```bash
npm run watch
```

Reload the Extension Development Host after grammar or snippet changes.
