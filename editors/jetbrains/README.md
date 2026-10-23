# JetBrains plugin skeleton (Phase 1.6a)

TextMate grammar reuse from `editors/vscode` / `editors/textmate`.

## Layout

```
editors/jetbrains/
  README.md          ← this file
  resources/
    META-INF/plugin.xml
```

Open as a Plugin DevKit project and point the TextMate bundle at the shared grammar. Full PSI/parser integration remains follow-up work after tree-sitter coverage lands.
