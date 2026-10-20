# Std.FS

**Std.FS** provides cross-platform paths, file I/O, directories, and change notifications.

```ll
import Std.FS
```

## Path

```ll
public struct Path: Equatable, Hashable, Sendable {
    public init(_ raw: String)
    public func join(_ components: String...) -> Path
    public var parent: Path? { get }
    public var fileName: String? { get }
    public func isAbsolute() -> Bool
}
```

Prefer `Path` over string concatenation — separators normalize per OS.

## File operations

```ll
public enum File {
    public static func read(_ path: Path) throws(FSError) -> Data
    public static func readText(_ path: Path, encoding: Encoding = .utf8) throws(FSError) -> String
    public static func write(_ path: Path, contents: Data, atomic: Bool = true) throws(FSError)
    public static func exists(_ path: Path) -> Bool
    public static func remove(_ path: Path) throws(FSError)
}
```

## Directory traversal

```ll
public struct DirectoryEntry {
    public var path: Path
    public var isDirectory: Bool
}

public func walk(_ root: Path) throws(FSError) -> DirectoryWalker
```

## File handles

```ll
public final class FileHandle: Reader, Writer {
    public static func open(_ path: Path, mode: OpenMode) throws(FSError) -> FileHandle
    public func close() throws(FSError)
}
```

## Watching

```ll
public struct FileWatcher {
    public init(root: Path) throws(FSError)
    public func nextEvent() async throws(FSError) -> FileEvent
}
```

## Temporary files

```ll
public func temporaryDirectory() throws(FSError) -> Path
public func createTemporaryFile(prefix: String = "ll") throws(FSError) -> (Path, FileHandle)
```

## See also

- [IO](io.md) — stream abstractions
- [Process](process.md) — cwd and pipes
