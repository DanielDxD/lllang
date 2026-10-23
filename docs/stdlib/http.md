# Std.Http

**Std.Http** HTTP client helpers and a durable path-routed server (v0.2).

```ll
import Std.Http
```

## Shipped MVP — client

```ll
Http.get(url: String) -> String
Http.post(url: String, body: String) -> String
```

## Shipped MVP — server

Persistent accept loop. Handler receives the request path and returns the response body:

```ll
public func handle(path: String) -> String {
    if path == "/health" {
        return "ok"
    }
    "404 Not Found"   // status becomes 404
}

public func main() {
    HttpServer.serve(8080, handle)
}
```

- Listens on `0.0.0.0:port`, parses `METHOD path HTTP/1.x`, calls `handle(path)`.
- Response: `text/plain`, `Connection: close`. Body exactly `404 Not Found` → HTTP 404.
- **TLS:** not built-in for v0.2 — put Caddy/nginx (or similar) in front. Native TLS remains ROADMAP open.

One-shot smoke helper (single request then exit):

```ll
HttpServer.listen(port: Int, body: String)
```

Prefer `serve` for real services. See `examples/http-server` and the `web` template.

## Not shipped

Full `HttpClient` / middleware / `HttpServer.route` DSL / TLS upgrades — design sketches only.
