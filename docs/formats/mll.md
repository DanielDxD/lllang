# `.mll` LLM metadata format

An **`.mll` file** holds **LLM-oriented metadata** about a package, module, or workspace. It is encoded in **TOON** (Token-Oriented Object Notation), the same structured format used by `Std.Toon` in application code. Humans and agents read `.mll` for context; the compiler does not require it for builds.

## Purpose

| Use case | How `.mll` helps |
|----------|------------------|
| Agent coding | Compact API summaries, pitfalls, conventions |
| `lang llm context` | Assemble prompts from repo + lockfile + `.mll` |
| MCP / agent templates | Declare tools, skills, and example flows |
| IDE (future) | Inline “explain this package” without scraping source |

`.mll` is **derived** from `.ll` / `.sbl` where possible (`lang llm gen`) and hand-tuned for narrative fields.

## TOON schema for `.mll`

Top-level object keys (all optional unless noted):

| Key | Type | Description |
|-----|------|-------------|
| `summary` | string | One-paragraph package purpose |
| `concepts` | array of objects | Domain terms agents must understand |
| `api` | object | Grouped public API index |
| `examples` | array of objects | Runnable or illustrative snippets |
| `pitfalls` | array of strings | Common mistakes |
| `conventions` | array of strings | Style / architectural rules |
| `tokens` | object | Budget hints for context assembly |
| `version` | string | **Required.** Schema version (`"1"`) |
| `package` | string | Package id (`acme.widgets`) |

### `concepts[]`

```toon
concepts:
  - id: arc
    title: Automatic Reference Counting
    body: Classes use ARC; structs are copy-by-value. Weak refs break cycles.
  - id: no-null
    title: Null safety
    body: Use T? and nil only for optionals; no null references.
```

### `api`

Hierarchical map of modules to short entries (not full `.sbl`):

```toon
api:
  Acme.Widgets:
    Button:
      kind: struct
      blurb: Primary action control with label and enabled state.
    Theme:
      kind: protocol
      blurb: Color and typography tokens.
```

### `examples[]`

```toon
examples:
  - title: Create a button
    lang: ll
    code: |
      import Acme.Widgets
      let b = Button(label: "Save", action: save)
```

### `pitfalls` / `conventions`

Plain string lists — optimized for bullet injection into system prompts.

### `tokens`

Hints for `lang llm context` when trimming context:

```toon
tokens:
  priority: high          # low | normal | high
  max_sections: 6
  prefer: [api, pitfalls, examples]
  omit_if_over_budget: [concepts]
```

## Full example

File: `acme.widgets.mll`

```toon
version: "1"
package: acme.widgets@2.1.0
summary: Cross-platform UI widgets with theme-aware styling and ARC-safe view models.

concepts:
  - id: view-model
    title: ViewModel
    body: Observable state lives in classes; views are structs that read models via bindings.

api:
  Acme.Widgets:
    Button:
      kind: struct
      blurb: Labeled control; invokes action closure on press.
    List:
      kind: struct
      blurb: Virtualized list; requires identifiable items.

examples:
  - title: Themed button
    lang: ll
    code: |
      import Acme.Widgets
      let btn = Button(label: "OK", theme: .primary) { submit() }

pitfalls:
  - Do not capture views strongly in model closures — use weak or struct callbacks.
  - Button actions must not block the UI thread; offload via Std.Async.

conventions:
  - PascalCase for widget types; camelCase for factories like primaryButton().
  - Public API only in namespace Acme.Widgets.*

tokens:
  priority: normal
  max_sections: 8
  prefer: [summary, api, pitfalls, examples]
```

## Generation workflow

```bash
lang llm gen              # all packages in workspace
lang llm gen -p acme.widgets
lang llm context > ctx.toon   # merge lockfile + mll + selected sources
lang llm explain Std.Json.decode
```

`gen` overwrites machine sections (`api`, `version`) but preserves hand-authored `pitfalls` / `conventions` if marked:

```toon
# user-retain: pitfalls, conventions
```

## Pitfalls (meta)

- **Do not** treat `.mll` as a second source of truth for types — always reconcile with `.sbl`.
- **Stale `.mll`** after API changes mislead agents; run `gen` in CI or pre-publish.
- **Secrets** never belong in `.mll`; it is committed and published.

## Conventions

- Filename: `<package-name>.mll` or `package.mll` at package root
- Check in alongside `Package.ll` for libraries meant for agent consumption
- MCP templates include an `mll/` folder with tool descriptors (see [MCP](../llm/mcp.md))

## TOON encoding rules (reminder)

- Indentation defines nesting (2 spaces per level)
- Strings with special characters may use `|` blocks
- Arrays use `-` list items or inline `[a, b]` for short lists
- See `Std.Toon` and [JSON/TOON skill](../../skills/) for encode/decode in apps

## See also

- [`.ll` source](ll.md)
- [LLM overview](../llm/overview.md)
- [Context generation](../llm/context-generation.md)
- [CLI: `lang llm`](../toolchain/cli-reference.md)
