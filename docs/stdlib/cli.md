# Std.CLI

**Std.CLI** parses command-line arguments, generates help, and supports shell completions. Pair with the `cli` project template ([Templates](../toolchain/templates.md)).

```ll
import Std.CLI
```

## Command tree

```ll
public struct CommandDescriptor {
    public var name: String
    public var summary: String
    public var subcommands: [CommandDescriptor]
    public var flags: [FlagDescriptor]
}

public struct FlagDescriptor {
    public var long: String
    public var short: Character?
    public var kind: FlagKind
    public var required: Bool = false
}

public enum FlagKind {
    case bool
    case string
    case int
    case stringArray
}
```

## Parsing

```ll
public struct ParsedArgs {
    public subscript(flag: String) -> FlagValue { get }
    public var positional: [String] { get }
    public func subcommand() -> String?
}

public enum CLI {
    public static func parse(_ descriptor: CommandDescriptor, args: [String] = Args.all) throws(CLIError) -> ParsedArgs
    public static func printHelp(_ descriptor: CommandDescriptor, to writer: Writer = StdIO.stdout)
}
```

## Declarative macros (attribute-driven)

```ll
@CLI.Root("mytool", summary: "Example CLI")
public struct MyTool {
    @CLI.Flag("--verbose", short: "v", "Enable verbose logging")
    public var verbose: Bool = false

    @CLI.Argument("input", "Input file path")
    public var input: Path

    @CLI.Run
    public func run() throws(AppError) {
        if verbose { Log.level = .debug }
        process(input)
    }
}
```

The compiler expands descriptors and wires `main` for executable packages.

## Completions

```ll
public enum Completions {
    public static func bash(_ descriptor: CommandDescriptor) -> String
    public static func zsh(_ descriptor: CommandDescriptor) -> String
    public static func fish(_ descriptor: CommandDescriptor) -> String
}
```

Generate via `mytool --completions bash`.

## See also

- [Env](env.md) — raw `Args`
- [Log](log.md) — verbose mode
