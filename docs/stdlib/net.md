# Std.Net

**Std.Net** exposes TCP/UDP sockets, addresses, and TLS configuration hooks used by HTTP and custom protocols.

```ll
import Std.Net
```

## Addresses

```ll
public struct IPAddress: Equatable, Hashable, Codable {
    public static func parse(_ text: String) throws(NetError) -> IPAddress
    public var isIPv4: Bool { get }
    public var isIPv6: Bool { get }
}

public struct SocketAddress {
    public init(ip: IPAddress, port: UInt16)
    public var ip: IPAddress { get }
    public var port: UInt16 { get }
}
```

## TCP

```ll
public final class TcpListener {
    public init(bind: SocketAddress) throws(NetError)
    public func accept() async throws(NetError) -> TcpStream
}

public final class TcpStream: Reader, Writer {
    public static func connect(to address: SocketAddress) async throws(NetError) -> TcpStream
    public func setReadTimeout(_ timeout: Duration?)
    public func close() throws(NetError)
}
```

## UDP

```ll
public final class UdpSocket {
    public init(bind: SocketAddress?) throws(NetError)
    public func send(to: SocketAddress, payload: Span<UInt8>) throws(NetError)
    public func receive(into buffer: inout [UInt8]) throws(NetError) -> (count: Int, from: SocketAddress)
}
```

## TLS

```ll
public struct TlsConfig {
    public init(verifyHost: Bool = true, roots: CertificateStore = .system)
    public static func client() -> TlsConfig
}

public extension TcpStream {
    public func upgradeTls(config: TlsConfig, serverName: String) async throws(NetError) -> TlsStream
}
```

## DNS

```ll
public func resolve(_ host: String, port: UInt16) async throws(NetError) -> [SocketAddress]
```

## See also

- [Http](http.md) — HTTP client on top of `TcpStream` / TLS
- [Async](async.md) — async socket operations
