namespace {{Name}}

import Std.IO

public func usage() -> String {
    "Usage: {{name}} [--help|--version] [name]"
}

public func version() -> String {
    "{{name}} 0.1.0"
}

/// Parse argv: 0=help, 1=version, 2=greet (arg index of name), 3=bare hello.
public func parseMode(argc: Int, arg1: String) -> Int {
    if argc <= 1 {
        return 3
    }
    if arg1 == "--help" || arg1 == "-h" {
        return 0
    }
    if arg1 == "--version" || arg1 == "-V" {
        return 1
    }
    2
}

public func greet(name: String) -> String {
    "Hello, \(name)!"
}

public func main() {
    let n = Args.count
    let arg1 = switch Args.get(1) {
        .some(v) => v,
        .none => "",
    }
    let mode = parseMode(n, arg1)
    if mode == 0 {
        print(usage())
        return
    }
    if mode == 1 {
        print(version())
        return
    }
    if mode == 2 {
        print(greet(arg1))
        return
    }
    print(greet("world"))
}
