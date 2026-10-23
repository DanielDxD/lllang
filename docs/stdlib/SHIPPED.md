# Stdlib freeze — v0.2 shipped surface

What **`lang` v0.2 guarantees** for application code. Anything else in design docs is a sketch until listed here.

Stdlib is still **compiler builtins + `runtime/lllang_rt.c`** (not separate published `.bll` modules).

## Guaranteed

| Area | Surface |
|------|---------|
| Core / IO | `print` / `eprint`, optionals, `Result` MVP, fatal/assert |
| String | concat, interp, trim/split/contains/substring, upper/lower/replacing, `[String].joined` |
| Arrays | `[Int]` literals, append, count, index; HOFs `map`/`filter`/`reduce`/`sorted`/`any`/`all` |
| Set | insert/contains/count; `union`/`intersection`/`difference` (string keys) |
| Dict | `[String:String]` get/set/contains/keys/count |
| Env | `@loadEnv`, typed `Env.*` getters (string/int/bool/float/double/decimal/…) |
| Identity | `uuid` / `ulid` / `nanoid` / `shortId` / `parse` |
| Crypto | `Crypto.sha256`, `HMAC.sha256` |
| JSON / TOON | encode/decode MVP used by examples |
| HTTP | `Http.get` / `Http.post`; **`HttpServer.serve(port, handle)`** (HTTP only) |
| Net | `TcpStream.connect` / read / write / close |
| Async | `Task` join, `async let` (zero-arg), `TaskGroup`, `Channel`, `WaitGroup`, `@run` (zero-arg free) |
| SQL | `Std.SQL` sqlite (incl. `:memory:`) |
| FFI | `@extern(cName:)` + auto-link `interop/**/*.c` |
| Regex | `Regex.isMatch` / `String.matches` |
| Testing | `@test` / `@suite` / `@expect` / `lang test` |

## Not guaranteed (v0.2)

- Native **TLS** / HTTPS (use a reverse proxy)
- Generic `[T]` HOFs beyond `[Int]`; Set of non-strings
- `AsyncSequence` / `for await` / full DAP debugger
- Phase **1.7** sketches under `tmp/1.2/` (WebSocket, Cron, unions, Html DSL, …)
- Multi-lang `@interop` beyond C; SQL dialects beyond sqlite
- Published `Std.*.bll` packages from a public registry

## CI freeze examples

`extensions`, `env-typed`, `identity`, `stdlib-surface`, `hofs`, `task-group`, `regex-smoke`, `interop-c`, `hmac-smoke`, `http-server` (curl smoke), `prebuilt-consumer`, `sql-memory` (when green), `unit-tests`.
