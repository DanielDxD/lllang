# Std.Http

**Std.Http** provides an async HTTP client, a lightweight server, and composable middleware.

```ll
import Std.Http
```

## Client

```ll
public struct HttpClient {
    public init(base: URL? = nil, timeout: Duration = .seconds(30))
    public func get(_ path: String, headers: [String: String] = [:]) async throws(HttpError) -> Response
    public func post(_ path: String, body: Data, contentType: String = "application/octet-stream") async throws(HttpError) -> Response
    public func send(_ request: Request) async throws(HttpError) -> Response
}

public struct Request {
    public var method: Method
    public var url: URL
    public var headers: [String: String]
    public var body: Data
}

public struct Response {
    public var status: Int
    public var headers: [String: String]
    public var body: Data
    public func json<T: Decodable>(_ type: T.Type) throws(HttpError) -> T
}
```

## URL

```ll
public struct URL: Equatable, Hashable, Codable {
    public init(_ string: String) throws(UrlError)
    public var scheme: String { get }
    public var host: String { get }
    public var path: String { get }
}
```

## Server

```ll
public typealias HttpHandler = (Request) async throws(HttpError) -> Response

public struct HttpServer {
    public init(bind: SocketAddress)
    public func route(_ method: Method, _ pattern: String, handler: @escaping HttpHandler)
    public func use(_ middleware: Middleware)
    public func run() async throws(HttpError)
}
```

## Middleware

```ll
public struct Middleware {
    public static func logging() -> Middleware
    public static func gzip() -> Middleware
    public static func cors(origins: [String]) -> Middleware
}
```

## Streaming bodies

Large uploads/downloads use stream types instead of materializing `Data`:

```ll
public protocol HttpBody {
    func write(to writer: Writer) async throws(HttpError)
}
```

## See also

- [Net](net.md) — transport layer
- [Json](json.md) — response decoding
