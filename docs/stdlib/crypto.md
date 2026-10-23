# Std.Crypto

**Std.Crypto** hashing, HMAC, and secure random helpers (builtins + C runtime).

```ll
import Std.Crypto
```

## Shipped MVP

```ll
Crypto.sha256(text: String) -> String          // lowercase hex
HMAC.sha256(key: String, data: String) -> String
Hash.sha256(text: String) -> String            // alias
Base64.encode / Hex helpers                    // where registered in prelude
SecureRandom                                   // via runtime where wired
```

See `examples/hmac-smoke` and `examples/stdlib-surface`.

## Design targets (not all shipped)

Generic `HMAC<Key: HashAlgorithm>`, BLAKE3, and ChaCha20-Poly1305 AEAD remain design sketches — prefer the free functions above until those types lower.
