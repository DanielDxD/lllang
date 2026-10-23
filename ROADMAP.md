# Roadmap

Phases 1–10 delivered a **vertical-slice language**: parse, typecheck, LLVM 22 codegen, ARC, concurrency MVP (thread-pool executor), sqlite `Std.SQL`, packages/registry, templates, and `lang test`/`fmt`/`pack`. Ordinary examples under `examples/` run end-to-end.

**What lowers today (MVP):** structs/classes/actors, primary constructors, inheritance/virtuals, optionals, arrays and string-keyed dictionaries, `#defer` / `#die`, closures (limited), generics instantiation, enum switch, ARC + weak/unowned, `@run` / Channel / WaitGroup / Task join, existentials, nested Codable, compile-time `$` / `#if` / `#declare`, and thin `Std.*` builtins backed by `runtime/lllang_rt.c`.

**Phase 1.1 closed polish gaps.** Phases **1.2–1.6** drain the former Later backlog as **MVPs** (TLS / SQL dialect #2 / multi-lang interop beyond C remain open). `tmp/1.2/*` is Phase **1.7** placement only.

### v0.2 acceptance *(usable CLI / service slice)*

1. Install prebuilt `lang` (or `cargo build -p lang --release` + pack).
2. `lang new cli|web -n demo` → `lang check` / `build` / `test` / `run`.
3. `HttpServer.serve(port, handle)` accepts requests in a loop; `curl /health` works (HTTP; TLS via reverse proxy).
4. Freeze examples in CI match [docs/stdlib/SHIPPED.md](docs/stdlib/SHIPPED.md).
5. Tag `v0.2.0` triggers [.github/workflows/release.yml](.github/workflows/release.yml) assets + `SHA256SUMS` (push tag when ready).

---

## Phase 1 — Spec, docs, syntax, LSP shell *(done)*

- [x] Language specification and design docs (English)
- [x] File formats: `.ll`, `.bll`, `.sbl`, `.mll`
- [x] CLI & package manager design (`lang`)
- [x] Standard library API surface (documented)
- [x] Examples and agent skills
- [x] `lllang-syntax` lexer/parser + diagnostics
- [x] Minimal LSP (`lang lsp`) + VS Code / web highlighting

## Phase 2 — Semantic analysis & LLVM (vertical slice) *(done)*

- [x] Name resolution and type checker (`lllang-sema`, diagnostics LL1xxx / LL2xxx)
- [x] HIR → MIR with ARC retain/release insertion (`lllang-mir`)
- [x] LLVM 22 codegen via `inkwell` (`lllang-llvm`) + C runtime (`runtime/lllang_rt.c`)
- [x] `lang check` with sema; `lang build` / `lang run` for native binaries
- [x] Examples: `hello`, `errors`, `shapes-oop`, `shop-solution` (path deps / `Solution.ll`)

### Phase 2.1 — tooling pass *(done)*

- [x] Formatter (`lang fmt`)
- [x] Fuller LSP (go-to-def, rename, inlay hints) on top of sema
- [x] Richer throwing ABI / COW collections / class vtables polish

## Phase 3 — Stdlib runtime & packages *(MVP done)*

- [x] Implement `Std.*` modules (Json, Toon, Async, Llm, …) — MVP for examples
- [x] Global package store + resolver + `Package.lock`
- [x] `.bll` packing (`lang pack`) and registry protocol (local store / search)
- [x] Cross-target builds (host + `--target` / `lang sdk` MVP)

## Phase 1.6.1 — Multi-target + portable `.bll` *(done)*

- [x] Go-style `--target` / `LANG_OS`+`LANG_ARCH` for official triples (darwin/arm64, linux/{amd64,arm64}, windows/{amd64,arm64})
- [x] BLL v2 portable LLVM bitcode (`package.bll`); finalize at consumer link; single lock `bll` hash
- [x] Per-triple runtime under `$LANG_HOME/toolchain/lib/<triple>/`
- [x] Docs/ADR 010; reject BLL v1 native-only blobs

## Phase 4 — DX polish & ecosystem *(MVP done)*

- [x] Full LSP (type-aware completion, code actions) — MVP: members after `.`, LL2005 `try` quick-fix
- [x] Tree-sitter grammar stub (`editors/tree-sitter-lllang`; JetBrains still optional later)
- [x] Registry client + file-backed mock (`lang publish` / `search`, `LANG_REGISTRY`; no public host yet)
- [x] MCP / agent templates (`lang new`) and `examples/mcp-echo`
- [x] Benchmarks MVP (`lang bench` + `docs/toolchain/benchmarks.md`)

---

## Phase 5 — Core language executes *(done — builtins + MIR/LLVM MVP)*

Without this, templates and stdlib have nothing solid to sit on. The compiler must **stop silently accepting** constructs it does not emit.

- [x] Explicit diagnostics for constructs that do not lower (no silent empty body / `Expr::Nil`)
- [x] Assignment stores for locals, fields, then subscripts
- [x] Arrays: literals, `append` / `count` / indexing, using existing `ll_array_*` in `runtime/lllang_rt.c`
- [x] `for` / ranges / `break` / `continue`
- [x] `#defer` (expression and block form, LIFO on scope exit) — see `tmp/macro_idea.ll`; align language guide (today documents keyword `defer`)
- [x] `#die "message"` — immediate abort with message (trap), distinct from library `fatalError`
- [x] Closures as values (no capture first, then captures)
- [x] Generics that instantiate (`Array<T>`, `Result<T, E>`, generic functions)
- [x] Enum `switch` with real tags, payloads, and exhaustiveness (`LL2008` beyond `Optional`)
- [x] Strings: concat via `ll_string_concat`; `contains` / `split` / `trim` / `substring`
- [x] Real ARC: retain on copy, release of fields, call `deinit` (`weak` / `unowned` in Phase 10)
- [x] Module-level `let` / `const`; short-circuit `and` / `or`

## Phase 6 — Stdlib baseline *(done — builtins + C runtime MVP)*

There are no real `Std.*` `.bll` modules yet — only compiler builtins and the C runtime. Imports such as `Std.FS` resolve to prelude builtins that link `ll_*` in `runtime/lllang_rt.c`.

- [x] `Std.Core`: `Result`, `fatalError`, `assert` / `precondition`
- [x] `Std.Text`, `Std.Collections` (`Array`, `Dictionary`, `Set`)
- [x] `Std.IO` beyond `print`: stdin, stderr, readers/writers
- [x] `Std.FS`, `Std.Env` (args and env vars), `Std.Process`, `Std.Time`
- [x] JSON / TOON with nested objects, arrays, and bools (nested keyed containers in Phase 10)
- [x] `try!`, statement `do` / `catch`, multiple `catch` clauses

## Phase 7 — Real project templates *(done)*

Templates are embedded in the `lang` binary (`include_dir`). `lang new` expands `{{name}}` / `{{Name}}` / `{{package}}`, writes a common skeleton, and optionally runs `git init` (`--no-git` to skip).

**Acceptance:** `lang new <template>` produces a tree where `lang check`, `lang build`, `lang run`, and (when tests exist) `lang test` pass — no “scaffold” output. `lang bench` times `@Bench` functions.

### 7a — Common skeleton (after Phase 5 core compiles)

- [x] `.gitignore` (`.lang/`), README with commands, `.editorconfig`, CI workflow
- [x] Placeholders `{{name}}`, `{{Name}}`, `{{package}}`; embed templates in the binary
- [x] `git init` with `--no-git`; fix `solution` path rename vs `Package.ll`
- [x] `console`, `lib`, `test`, `solution` as runnable projects (small tested API, `defaultProjects`, README)

### 7b — After matching stdlib

- [x] `cli` with real argument parsing + test
- [x] `web` with route, `assets/`, + test
- [x] `agent` that runs with a mock provider and a real tool
- [x] `mcp` with stdio loop that lists and calls `echo`
- [x] `bench` whose function is compiled and timed by `lang bench`

## Phase 8 — Ship an application *(done — builtins + C runtime MVP)*

- [x] Inheritance layout with base fields, `super`, `override` / `sealed` checks
- [x] Protocols as generic constraints with witnesses (existential values: Phase 10)
- [x] Full optionals: `??`, `if let`, chaining, force-unwrap
- [x] Real `Std.Http` client, `Std.Net`, `Std.Log`, `Std.CLI`, `Std.Sync`, `Std.Crypto`
- [x] FFI `unsafe` to call C
- [x] `#deprecated "message"` compiler warning — see `tmp/macro_idea.ll`; align guide (today `@deprecated(...)`)
- [x] Testing finish for `tmp/unit_test_idea.ll`: accept `parallel = false` as in the sketch; `@throws`; fixtures (`@suite` / `@theory` / `@values` / `@expect` already run)

## Phase 9 — Trustworthy toolchain *(done)*

- [x] Typecheck via `.sbl` and link `.bll` (path deps may still recompile source; store/registry deps use stubs + objects)
- [x] `lang add` edits `Package.ll` and fetches blobs; `lang publish` publishes what `lang pack` built; hash mismatch fails
- [x] Incremental builds and `lang watch`
- [x] `lang build --target` fails for non-host until a sysroot exists under `LANG_HOME/sdk`
- [x] Package-wide LSP; cross-file go-to-def / rename
- [x] `lang bench` executes `@Bench` and reports ms (same harness model as `lang test`)
- [x] Repo CI (`cargo test`, `lang test`, `lang fmt --check`) and installable tarball with runtime (`scripts/pack-release.sh`)
- [x] Compile-time `$` constants — see `tmp/system_constants.ll`: `$OS`, `$ARCH`, `$DEBUG`, `$RELEASE`, `$VERSION` (app), `$SDK_VERSION` (toolchain), `$BUILD_TIME`, `$BUILD_NUMBER`, `$BUILD_HASH`, `$BUILD_TAG`, `$BUILD_TYPE`, `$BUILD_ENV`
- [x] `#declare $NAME "value"` user compile-time constants (e.g. `#declare $SQL_DIALECT "sqlite"`)
- [x] `#if $DEBUG { ... } else { ... }` on system and `#declare`d constants — see `tmp/macro_idea.ll`

## Phase 10 — Concurrency & advanced types *(done — thread-pool executor MVP)*

- [x] Real `async` / `await` executor, `Task` / `TaskGroup` / `Channel`
- [x] `@run task()` fire-and-forget parallelism, channels, wait groups — see `tmp/parallel_idea.ll` (alongside structured async; does not replace it)
- [x] `actor`, cancellation, `Sendable`, and `$MAIN_ACTOR`
- [x] `Std.SQL` — see `tmp/sql.ll`: `SQLManager(dialect: .sqlite, path)`, `connect`, `execute`, close with `#defer`; sqlite first (incl. `":memory:"`); other dialects later; needs real `await` and `#defer`
- [x] `weak` / `unowned` and side table
- [x] Protocol existentials as values and extensions
- [x] `Decimal` and `UInt8`…`Float64` ladder
- [x] Codable with containers (not only field schemas)

## Phase 1.1 — Language polish *(after Phase 10)*

Close the gap between design docs and what ordinary programs can write.
Keep WASM, DAP/REPL, JetBrains, public registry, and HTTP server/TLS in **Later**.

**Acceptance:** design-claimed day-to-day syntax either works end-to-end or emits a clear diagnostic;
`tmp/1.1/*.ll` sketches either run under `examples/` or are explicitly deferred in Later.

### 1.1a — Syntax & sema

- [x] Register and lower `extension` methods (incl. `where`); `examples/extensions` from `tmp/1.1/extensions.ll`
- [x] Computed properties (`=>` / `get`/`set`) and observers (`willSet`/`didSet`)
- [x] Access-control ADR: one story for `fileprivate` vs `protected` and `open`; implement + docs ([ADR 009](docs/design/adr/009-access-control.md))
- [x] `private(set)` (parsed; setter narrowing accepted; visibility diagnostics MVP)
- [x] Casts: `as` / `as?` / `as!` with widening/narrowing from the types guide
- [x] `is` checks with optional binding (`if shape is Circle c`) — static MVP
- [x] Compound assignment; inclusive ranges `...`; richer `for` patterns (at least `(k, v)` over dictionaries)
- [x] Array literals for user struct/enum/class element types (pointer arrays); dict literals still `[String: String]` MVP
- [x] Multi-parameter and block closures with captures (scope clone; no heap capture box yet)
- [x] Real `Result<T, E>` without Int erasure (`Ty::Result`; Any-shaped payloads)
- [x] Multiple typed `catch` clauses; scope-accurate `#defer` (block via `Stmt::Scope`)
- [x] Custom `subscript` / `operator` declarations rejected with LL3001 → **Later**

### 1.1b — Concurrency polish

- [x] Honest `await`: join Task / `async let`; spawn+join zero-arg `async` calls; warn on no-op await
- [x] `async let` MVP (zero-arg func/ref); cancel remaining on early exit best-effort
- [x] `TaskGroup` / `withTaskGroup` → trimmed in `docs/stdlib/async.md` (**Later**)
- [x] `CancellationToken` polish (`CancellationToken()` ctor sugar; `create`/`cancel`/`throwIfCancelled`; `examples/cancellation` — no `fn.cancel()`)
- [x] Stronger `Sendable` / actor isolation diagnostics; `@run` zero-arg free funcs (broader `@run` Later)
- [x] Example covering cancel + actor (`examples/actors-cancel`); MainActor drain vs UI loop in `docs/stdlib/async.md`

### 1.1c — Stdlib surface (builtins → usable APIs)

- [x] Collections: `[String].joined(sep)` MVP; `map` / `filter` / `reduce` / `sorted` / `any`/`all` / Set algebra → **Later** (closures do not lower to runtime HOFs yet)
- [x] String: `uppercased` / `lowercased` / `replacing` / `joined` (via array); UTF-8 / grapheme views stay Later
- [x] `@loadEnv` (.env) + typed `Env` getters with default and optional overloads:
  - `string`, `integer`/`int`, `bool` (`true`/`false`/`1`/`0`/`yes`/`no`)
  - `float` / `double` (`Float`/`Float32`, `Double`/`Float64`)
  - `decimal` (same parse path as `Decimal("…")`)
  - peers: `uint`, `int64` aligned with the numeric ladder
- [x] Prefer API name `Env` (`Environment` kept as alias)
- [x] `Std.Identity` uuid/ulid/nanoid MVP (`examples/identity`; parse/equals/shortId/snowflake Later)
- [x] Math free functions + `Decimal` add/sub/mul (`examples/stdlib-surface`)
- [x] Crypto: Base64/Hex + secure random + sha256; **HMAC → Later** (no runtime helper yet)
- [x] Process start/wait; `Log.debug` (structured metadata fields → Later)
- [x] Thin `@interop(.c)` deferred → **Later** (keep `@extern` / `unsafe`)
- [x] Regex / Validator / KeyPath rules → **Later** (not pretended in overview)

### 1.1d — Tooling / DX

- [x] Code actions for common LL3001/LL200x (casts, `try`, missing override, empty array type)
- [x] LSP: completion for extension members; hover shows deprecation/`throws` (MVP)
- [x] Promote working `tmp/1.1` sketches to `examples/`; CI `lang check`/`run` (Phase 1.1 examples)
- [x] Tree-sitter: cover extensions, `#defer`, actors (full grammar still Later)

### 1.1e — Docs & ROADMAP hygiene

- [x] Rewrite ROADMAP intro to match Phases 5–10 reality
- [x] Update `AGENTS.md` phase notes through 10 + 1.1
- [x] Fix `docs/language/15-namespaces-modules.md` (Package.ll, remove TOML sample)
- [x] Align philosophy / design table with shipped concurrency and LLM status
- [x] Sweep stdlib docs for `associatedtype` / `some` / `any` and “planned” wording
- [x] Extend “Ideas from tmp/” with `tmp/1.1/*` placements

## Phase 1.2 — Language completion *(done — MVP)*

Drain former Later language foundations. Docs must match shipped surface (Wave 0 honesty).

- [x] Heap closure environments for non-literal / escaping captures (leading-param env + `FuncRef.captures`)
- [x] Array/Set HOFs: `map` / `filter` / `reduce` / `sorted` / `any`/`all` / `union`/`intersection`/`difference` (`[Int]` + String Set)
- [x] `TaskGroup` create / `add` / `wait` (`examples/task-group`)
- [x] `@run` zero-arg free funcs + valued `async let` spawn (instance-method `@run` still limited)
- [x] Custom `subscript` / `operator` — still rejected with LL3001 (parser keywords reserved); use methods
- [x] Thin `@interop(.c)` via compile+link of `interop/**/*.c` + `@extern` (`examples/interop-c`); multi-lang → 1.5
- [x] Regex MVP (`Regex.isMatch` / `String.matches`); Validator / KeyPath stay thin

## Phase 1.3 — Stdlib completion *(done — MVP)*

- [x] Crypto HMAC-SHA256; `Log.debugFields`; Identity parse / shortId
- [x] UTF-8 via existing String; grapheme cluster views deferred (documented)
- [x] `runBlocking` no-op MVP + `with_timeout_ms` helper; AsyncSequence / `for await` → 1.7
- [x] `Arena.create` MVP; streaming/SIMD JSON still measured Later

## Phase 1.4 — Network *(done — MVP slice)*

- [ ] TLS (system/OpenSSL/Schannel) — still open
- [x] `HttpServer.serve(port, handle)` durable accept loop + one-shot `listen` (`examples/http-server`, `web` template)
- [x] `TcpStream` + HTTP serve; DNS/TLS richer APIs remain open (HTTPS via reverse proxy for v0.2)
- [x] CI freeze + `http-server` curl smoke / `hofs` / `task-group` / `interop-c`

## Phase 1.5 — Platform services *(done — MVP)*

- [ ] SQL dialect beyond sqlite — still open (sqlite default)
- [x] LLM: OpenAI-compatible path via env `LANG_LLM_URL` + mock (`LANG_LLM_MOCK`); see runtime
- [x] Multi-lang `@interop`: C via `interop/`; Rust/Swift/Python still open
- [x] Public registry host (`registry/host/server.py` file-backed)

## Phase 1.6a — Tooling *(done — MVP)*

- [x] `lang debug` DAP probe shell
- [x] `lang repl`
- [x] Tree-sitter grammar present; JetBrains plugin skeleton (`editors/jetbrains`)
- [x] `lang new --template-path`
- [x] `lang doc` / `lang update` / `lang llm` shipped as MVPs (no longer silent stubs)

## Phase 1.6b — Distribution *(done — docs + host)*

- [x] WASM / WASI documented policy (`docs/toolchain/wasm.md`); sysroot still required
- [x] JetBrains skeleton publishable as TextMate-only MVP
- [x] Registry host demo-ready (`python3 registry/host/server.py`)
## Phase 1.7 candidates *(post-Later — placement only)*

Sketches under `tmp/1.2/` — do not block 1.2–1.6:

| Sketch | Items |
|--------|--------|
| `tmp/1.2/websocket.ll` | `Std.WebSocket`, `@gateway` / `@event` |
| `tmp/1.2/cron.ll` | `Std.Cron`, `@crontask` |
| `tmp/1.2/union.ll` | Union types `A \| B` |
| `tmp/1.2/transaction.ll` | `Std.Transaction` |
| `tmp/1.2/llm.ll` | Streaming / `@schema` LLM surface |
| `tmp/1.2/async_loop.ll` | `for await` / AsyncSequence sugar |
| `tmp/1.2/sensitive.ll` | `@sensitive` redaction |
| `tmp/1.2/html_structure.ll` | `Std.Html` DSL |
| `tmp/1.2/realtime_compute.ll` | `@compute` compile-time eval |

---

## Ideas from `tmp/` (placement)

| Sketch | Items | Phase |
|--------|--------|-------|
| `tmp/macro_idea.ll` | `#defer`, `#die` | 5 |
| `tmp/macro_idea.ll` | `#deprecated` | 8 |
| `tmp/macro_idea.ll` | `#if $DEBUG { } else { }` | 9 |
| `tmp/system_constants.ll` | `$OS`, `$ARCH`, `$DEBUG`, `$RELEASE`, `$VERSION`, `$SDK_VERSION`, `$BUILD_*`, `#declare` | 9 |
| `tmp/system_constants.ll` | `$MAIN_ACTOR` | 10 |
| `tmp/unit_test_idea.ll` | suite titles, `@theory`/`@values`, `@expect` (mostly done); `parallel = false`, `@throws`, fixtures | 8 |
| `tmp/parallel_idea.ll` | `@run`, channels, wait groups | 10 |
| `tmp/sql.ll` | `SQLManager` sqlite, `connect` / `execute`, `#defer` | 10 |
| `tmp/1.1/extensions.ll` | `extension` methods | 1.1a |
| `tmp/1.1/cancellation.ll` | CancellationToken API polish | 1.1b |
| `tmp/1.1/env.ll` | `@loadEnv`, typed Env (string/int/bool/float/double/decimal/…) | 1.1c |
| `tmp/1.1/identifiable.ll` | `Std.Identity` | 1.1c |
| `tmp/1.1/interop.ll` | `@interop` C MVP | 1.2 |
| `tmp/1.1/stdlib_full.ll` | Collections/String/Math/Crypto/Time/Log/Process | 1.1c |
| `tmp/1.2/*` | WebSocket, Cron, unions, Transaction, Html, `@sensitive`, `@compute`, `for await` | 1.7 candidates |

---

## Non-goals

- Garbage collection
- Dynamic typing / optional types that erase null safety
- Shipping dependency *source* into projects (libraries remain prebuilt `.bll`)
