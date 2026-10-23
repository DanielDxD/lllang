namespace {{Name}}

import Std.IO
import Std.Llm

public func listTools() -> String {
    "tools: echo"
}

public func handleLine(line: String) -> String {
    let trimmed = line.trim()
    if trimmed == "list" {
        return listTools()
    }
    if trimmed.contains("call echo") {
        let parts = trimmed.split(" ")
        if parts.count > 2 {
            return echo(parts[2])
        }
        return echo("")
    }
    if trimmed == "quit" || trimmed == "exit" {
        return "bye"
    }
    "unknown command"
}

/// Default: demo list + echo. Pass `--stdio` for a readLine loop.
public func main() {
    var tools = ToolRegistry()
    tools.register(echo.self)
    let arg1 = switch Args.get(1) {
        .some(v) => v,
        .none => "",
    }
    if arg1 == "--stdio" {
        print("mcp ready — commands: list | call echo <text> | quit")
        while true {
            let lineOpt = readLine()
            let line = switch lineOpt {
                .some(v) => v,
                .none => "quit",
            }
            let out = handleLine(line)
            print(out)
            if out == "bye" {
                break
            }
        }
        return
    }
    print(listTools())
    print(handleLine("call echo ping"))
}
