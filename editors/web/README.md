# LLLang web highlighting

Lightweight syntax highlighting for documentation sites and demos — no VS Code required.

## Contents

| Path | Use |
|------|-----|
| `shiki/lllang.json` | TextMate grammar JSON for [Shiki](https://shiki.style/) |
| `highlightjs/lllang.js` | [highlight.js](https://highlightjs.org/) language definition |
| `demo.html` | Standalone demo page |

## Shiki

Register the bundled grammar (same rules as the VS Code extension):

```typescript
import { createHighlighter } from "shiki";

const highlighter = await createHighlighter({
  langs: ["lllang"],
  themes: ["github-dark"],
  langAlias: { ll: "lllang" },
});

// Load editors/web/shiki/lllang.json and pass to loadLanguage if needed
```

Copy `shiki/lllang.json` into your app or import it from this repo path.

## highlight.js

```html
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script src="./highlightjs/lllang.js"></script>
<script>hljs.highlightAll();</script>
```

Use `class="language-lllang"` on `<pre><code>` blocks.

## Demo

Open `demo.html` in a browser (or serve the `editors/web` directory with any static file server).
