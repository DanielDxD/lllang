# WASM target (Phase 1.6b MVP)

LLLang can emit a **language subset** for `wasm32-wasi` when a WASI sysroot is installed under `$LANG_HOME/sdk/wasm32-wasi`.

```bash
lang sdk install wasm32-wasi   # downloads/stubs sysroot when available
lang build --target wasm32-wasi
```

**Shipped today:** host triple only; `--target wasm32-wasi` fails with a clear message until the sysroot exists (same policy as other cross targets). Runtime ARC/strings/arrays are portable C11; sockets/sqlite/HTTP are **not** available under WASI in this MVP.

See [ROADMAP](../../ROADMAP.md) Phase 1.6b.
