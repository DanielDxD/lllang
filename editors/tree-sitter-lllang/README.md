# tree-sitter-lllang

Experimental [Tree-sitter](https://tree-sitter.github.io/) grammar stub for LLLang.

**Source of truth for editor highlighting** remains the TextMate grammar:

- [`../vscode/syntaxes/lllang.tmLanguage.json`](../vscode/syntaxes/lllang.tmLanguage.json)
- Shiki: [`../web/shiki/lllang.json`](../web/shiki/lllang.json)

This stub covers a minimal subset (imports, namespaces, funcs, structs, comments) for Neovim experiments. It is not published and is not wired into CI.

```bash
# optional local generate (requires tree-sitter CLI)
npx tree-sitter generate
```
