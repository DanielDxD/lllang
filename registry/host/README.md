# LLLang registry host (Phase 1.5 MVP)

Minimal file-backed HTTP registry for local demos.

```bash
# From repo root — serves ./registry/data on :4873
python3 registry/host/server.py
```

Endpoints:

| Method | Path | Behavior |
|--------|------|----------|
| GET | `/health` | `ok` |
| GET | `/v1/search?q=` | JSON list of packages under `registry/data/` |
| GET | `/v1/packages/:id/:version/package.sbl` | Serve `.sbl` |
| GET | `/v1/packages/:id/:version/targets/:triple.bll` | Serve `.bll` |
| PUT | `/v1/packages/:id/:version/...` | Publish blob into `registry/data/` |

Point the client with `LANG_REGISTRY=http://127.0.0.1:4873`.
