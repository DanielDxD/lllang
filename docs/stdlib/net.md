# Std.Net

**Std.Net** socket primitives. Import is optional for free builtins resolved via the prelude.

```ll
import Std.Net
```

## Shipped MVP

```ll
TcpStream.connect(host: String, port: Int) -> TcpStream?
stream.read() -> String
stream.write(data: String)
stream.close()
```

Backed by `ll_tcp_*` in `runtime/lllang_rt.c` (Winsock on Windows).

## Also shipped — HTTP server

Prefer [`HttpServer.serve`](http.md) for path-routed HTTP (built on sockets in the runtime). TLS remains unshipped — reverse-proxy for HTTPS.

## Not shipped yet

- `IPAddress` / `SocketAddress` rich types
- `TcpListener.accept` as a separate API
- `UdpSocket`
- `TlsConfig` / `upgradeTls` / `TlsStream`
- Async DNS `resolve`

## See also

- [Http](http.md) — HTTP on top of TCP (server Phase 1.4)
- [Async](async.md) — tasks / channels
- [ROADMAP](../../ROADMAP.md) — Phase 1.4
