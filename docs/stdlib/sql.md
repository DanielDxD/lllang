# Std.SQL

**Std.SQL** provides async database access. Phase 10 ships a **sqlite** MVP (including `":memory:"`). Other dialects are rejected at runtime until a later phase.

```ll
import Std.SQL
```

## Quick start

```ll
public async func main() {
    let connection = await SQLManager.open(":memory:")
    #defer connection.close()
    let result = await connection.execute("SELECT 1")
    print(result)
}
```

## API (MVP)

```ll
public enum SQLManager {
    /// Open a sqlite database. `path` may be `":memory:"`.
    public static func open(_ path: String) async -> SQLConnection
}

public class SQLConnection {
    public func execute(_ sql: String) async -> String
    public func close()
}
```

Under the hood, `await` joins a thread-pool task while sqlite runs on a worker thread. Prefer `#defer connection.close()` so connections are always released.

## Dialects

| Dialect | Status |
|---------|--------|
| sqlite | Supported |
| postgres / mysql | Later |

## Related

- Language: [Concurrency](../language/12-concurrency.md), [`#defer`](../language/05-statements.md)
- Example: `examples/sql-memory`
