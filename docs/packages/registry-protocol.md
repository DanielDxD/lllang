# Registry protocol

LLLang registries expose an **HTTP index** of package metadata and **content-addressed blobs** for `.sbl`, `.bll`, and packed tarballs. The **`lang`** client implements this protocol for `add`, `restore`, `update`, `search`, and `publish`.

Default public registry URL (planned): `https://registry.lang.dev/` — override with `LANG_REGISTRY`.

## Design goals

- **Simple caching** — immutable blobs keyed by hash
- **CDN-friendly** — GET-heavy, no session for downloads
- **Reproducible installs** — lockfile stores hashes, not just versions
- **Multi-target** — index lists available `.bll` per triple

## Index API

### Search packages

```http
GET /v1/search?q=widgets&limit=20
```

Response (`application/json`):

```json
{
  "results": [
    {
      "name": "acme.widgets",
      "description": "UI widgets",
      "latest": "2.1.0",
      "downloads": 10452
    }
  ]
}
```

### Package metadata

```http
GET /v1/packages/{name}
```

Returns version list and owners.

### Version manifest

```http
GET /v1/packages/{name}/{version}/manifest.json
```

Example:

```json
{
  "name": "acme.widgets",
  "version": "2.1.0",
  "abi": 2,
  "license": "MIT",
  "sbl": {
    "hash": "blake3:abc123...",
    "size": 4096,
    "url": "/v1/blobs/blake3/abc123..."
  },
  "targets": [
    {
      "triple": "aarch64-apple-darwin",
      "bll": {
        "hash": "blake3:def456...",
        "size": 120000,
        "url": "/v1/blobs/blake3/def456..."
      }
    },
    {
      "triple": "x86_64-unknown-linux-gnu",
      "bll": {
        "hash": "blake3:789abc...",
        "size": 118000,
        "url": "/v1/blobs/blake3/789abc..."
      }
    }
  ],
  "dependencies": {
    "std.io": "^0.3"
  },
  "yanked": false
}
```

`sbl.url` and `bll.url` may be absolute CDN URLs.

### Yanked versions

`yanked: true` — existing lockfiles may still restore; `lang add` without exact pin warns and skips unless `--allow-yanked`.

## Blob storage

```http
GET /v1/blobs/{algorithm}/{digest}
```

- **Algorithm:** `blake3` (primary), `sha256` (legacy interop)
- **Body:** raw bytes of `.sbl`, `.bll`, or `.tar.zst` pack
- **Headers:** `Content-Type: application/octet-stream`, `Cache-Control: immutable`

Upload (publish):

```http
PUT /v1/blobs/{algorithm}/{digest}
Authorization: Bearer <token>
Content-Type: application/octet-stream
```

Idempotent — duplicate hash returns 204.

## Publish flow

```bash
lang pack --output acme.widgets-2.1.0.tar.zst
lang publish --registry https://registry.lang.dev/
```

Client steps:

1. Compute hashes for local pack contents
2. Upload missing blobs (parallel)
3. `POST /v1/packages/{name}/{version}` with manifest JSON
4. Registry validates ABI, semver, and signature (when enabled)

```http
POST /v1/packages/acme.widgets/2.1.0
Authorization: Bearer <token>
Content-Type: application/json

{ ... manifest ... }
```

Success: `201 Created` with canonical URL.

## Authentication

| Method | Use |
|--------|-----|
| Bearer token | CI publish (`LANG_REGISTRY_TOKEN`) |
| OAuth device flow | Interactive `lang login` (planned) |

Read operations (search, download) are anonymous for public packages.

## Lockfile integration

`lang restore` maps each package entry in `Package.lock` to:

1. `GET manifest.json` (or cache) — verify version metadata
2. `GET` each blob hash if absent from store
3. Verify on-disk hash matches lock entry

Network failure with warm store: succeed if all hashes present (`--offline`).

## Mirrors and private registries

Private registries implement the same paths. Index may require auth:

```http
GET /v1/packages/private.lib/1.0.0/manifest.json
Authorization: Bearer <read-token>
```

## Integrity

- Manifest references blob hashes; client recomputes after download
- Optional **package signature** in manifest (`signatures` array) verified against trusted keys
- `.bll` HASH section must match `.sbl` footer (see [`.bll`](../formats/bll.md))

## Error responses

| HTTP | Meaning |
|------|---------|
| 404 | Unknown package/version |
| 409 | Version already published |
| 422 | Manifest invalid (ABI, missing targets) |
| 401 / 403 | Auth failure |

Body:

```json
{
  "error": "manifest_invalid",
  "message": "bll hash mismatch for triple x86_64-unknown-linux-gnu"
}
```

## See also

- [Package manager](package-manager.md)
- [Versioning & ABI](versioning-and-abi.md)
- [CLI publish / search](../toolchain/cli-reference.md)
