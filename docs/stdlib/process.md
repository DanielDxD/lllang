# Std.Process

**Std.Process** runs child processes, connects pipes, and reports exit status.

```ll
import Std.Process
```

## Command

```ll
public struct Command {
    public init(_ executable: Path, args: [String] = [])
    public var environment: [String: String] { get set }
    public var workingDirectory: Path? { get set }
    public func stdout(_ pipe: Pipe?) -> Command
    public func stderr(_ pipe: Pipe?) -> Command
    public func spawn() throws(ProcessError) -> ChildProcess
}
```

## Running to completion

```ll
public struct ProcessOutput {
    public var exitCode: Int32
    public var stdout: Data
    public var stderr: Data
}

public extension Command {
    public func run() throws(ProcessError) -> ProcessOutput
    public func runCollectingOutput() async throws(ProcessError) -> ProcessOutput
}
```

## Child lifecycle

```ll
public final class ChildProcess {
    public func wait() throws(ProcessError) -> Int32
    public func kill(signal: Signal = .term) throws(ProcessError)
    public var pid: Int { get }
}
```

## Current process

```ll
public enum Process {
    public static var pid: Int { get }
    public static func exit(_ code: Int32) -> Never
}
```

## See also

- [Env](env.md) — inherited environment
- [IO](io.md) — pipe types
