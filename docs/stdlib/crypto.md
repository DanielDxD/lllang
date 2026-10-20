# Std.Crypto

**Std.Crypto** provides hashing, message authentication, AEAD encryption, and cryptographically secure random bytes.

```ll
import Std.Crypto
```

## Secure random

```ll
public enum SecureRandom {
    public static func bytes(count: Int) throws(CryptoError) -> Data
    public static func uuidV4() -> UUID
}
```

## Hashing

```ll
public enum SHA256 {
    public static func hash(_ data: Span<UInt8>) -> Data
    public static func hashString(_ text: String) -> Data
}

public enum BLAKE3 {
    public static func hash(_ data: Span<UInt8>) -> Data
}
```

## HMAC

```ll
public struct HMAC<Key: HashAlgorithm> {
    public init(key: Data)
    public mutating func update(_ data: Span<UInt8>)
    public func finalize() -> Data
}
```

## AEAD

```ll
public struct ChaCha20Poly1305 {
    public init(key: Data) throws(CryptoError)
    public func seal(_ plaintext: Data, nonce: Data, additionalData: Data = Data()) throws(CryptoError) -> Data
    public func open(_ combined: Data, nonce: Data, additionalData: Data = Data()) throws(CryptoError) -> Data
}
```

## Constant-time comparison

```ll
public func constantTimeEqual(_ a: Data, _ b: Data) -> Bool
```

Use for MAC/tag verification — never compare secrets with `==` on strings.

## Key derivation

```ll
public enum PBKDF2 {
    public static func derive(password: Data, salt: Data, iterations: Int, length: Int) throws(CryptoError) -> Data
}
```

## See also

- [IO](io.md) — `Data` buffers
- [Net](net.md) — TLS certificate stores
