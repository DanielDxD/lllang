# Context generation

The **`lang llm`** commands produce and merge LLM-oriented context from your workspace: `.mll` files, lockfiles, and optionally selected sources.

**Format spec:** [`.mll`](../formats/mll.md) · **Encoding:** [Std.Toon](../stdlib/toon.md)

## Commands

| Command | Output | Purpose |
|---------|--------|---------|
| `lang llm gen` | Updates `*.mll` | Extract public API summaries from `.ll` / `.sbl` |
| `lang llm context` | TOON (stdout or file) | Bundle for system/developer prompts |
| `lang llm explain` | Plain text | Heuristic symbol/module summary |

```bash
lang llm gen
lang llm gen -p acme.widgets
lang llm context --max-tokens 120000 --include-sources > ctx.toon
lang llm explain Acme.Widgets.Button
```

CLI details: [CLI reference](../toolchain/cli-reference.md#lang-llm).

## `lang llm gen`

Walks parsed sources (semantic enrichment in later toolchain phases) and writes or refreshes `.mll` adjacent to packages.

**Preserves** user-authored sections when marked:

```toon
# user-retain: pitfalls, conventions
```

**Overwrites** machine-owned keys: `version`, `package`, `api` index derived from exports.

### Typical CI hook

```bash
lang llm gen && git diff --exit-code '*.mll'
```

Fails the build when agents would see stale API blurbs.

## `lang llm context`

Assembles a single TOON document for injection into agent prompts.

### Inputs (default order)

1. Workspace root `package.mll` (if present)
2. Each dependency’s `.mll` from the lockfile (respecting `tokens.priority`)
3. Optional `--include-sources` public `.ll` excerpts (signatures + doc comments only)
4. Manifest metadata from `Package.ll` (name, products, targets)

### Trimming (`tokens` in `.mll`)

Per-package hints control budget:

```toon
tokens:
  priority: high
  max_sections: 6
  prefer: [api, pitfalls, examples]
  omit_if_over_budget: [concepts]
```

Global cap: `--max-tokens N` (approximate tokenizer heuristic until provider-specific hooks land).

### Output shape

Top-level TOON object:

```toon
version: "1"
workspace: my.app@0.3.0
generatedAt: 2026-10-20T14:00:00Z
packages:
  - id: my.app
    summary: ...
    api: ...
  - id: acme.widgets@2.1.0
    summary: ...
sections:
  - kind: pitfalls
    source: acme.widgets
    lines: [...]
```

Convert to JSON for non-TOON consumers:

```bash
lang llm context | ll-toon-json   # planned helper; use Std.Toon.toJson in apps today
```

## `.mll` authoring checklist

| Field | Agent value |
|-------|-------------|
| `summary` | One-paragraph orientation |
| `concepts` | Domain vocabulary |
| `api` | Quick symbol index (not a replacement for `.sbl`) |
| `examples` | Runnable `lang: ll` snippets |
| `pitfalls` | Footguns and thread-safety notes |
| `conventions` | Naming and architecture rules |

Full schema: [formats/mll.md](../formats/mll.md).

## Runtime loading

Applications can load the same material without shelling out:

```ll
import Std.Llm

let ctx = try LlmContext.loadMll(Path("acme.widgets.mll"))
let merged = try await LlmContext.mergePackageContext("my.app")
```

## Pitfalls

- **Stale `.mll`** misleads agents after API changes — run `gen` before publish.
- **Do not** treat `.mll` as type truth — reconcile with `.sbl` exports.
- **Large `--include-sources`** blows token budgets; prefer `api` + `pitfalls` first.

## See also

- [Overview](overview.md)
- [Tools & structured output](tools-and-structured-output.md)
