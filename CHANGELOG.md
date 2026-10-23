# Changelog

## 0.2.0 — usable CLI / service slice

### Highlights

- Installers download GitHub Release binaries (`scripts/install.sh` / `install.ps1`); end users need clang + sqlite, not LLVM 22.
- Durable **`HttpServer.serve(port, handle)`** accept loop with path routing; `web` template listens for real. TLS via reverse proxy (native TLS still open).
- Language polish 1.1–1.2: extensions, casts, Env/Identity, `[Int]` HOFs, closure captures, TaskGroup, Regex, `@extern` + `interop/*.c`.
- Stdlib freeze contract: [docs/stdlib/SHIPPED.md](docs/stdlib/SHIPPED.md).
- Tooling MVPs: `lang repl`, `lang doc`/`update`/`llm`, `--template-path`, registry host demo (`registry/host/server.py`).

### Tag / release

```bash
git tag v0.2.0
git push origin v0.2.0   # triggers .github/workflows/release.yml
```

Do not push the tag until maintainers are ready; packing locally: `cargo build -p lang --release && ./scripts/pack-release.sh`.
