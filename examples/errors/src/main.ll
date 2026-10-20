namespace ErrorsDemo

import Std.IO

public enum ParseError: Error {
    empty,
    notANumber(text: String),
}

public func parseInt(text: String) throws(ParseError) -> Int {
    guard !text.isEmpty else { throw .empty }
    guard let value = Int.from(text) else { throw .notANumber(text) }
    return value
}

public func main() {
    let ok = try? parseInt("42")
    let soft = parseInt("nope") catch e => 0
    let maybe = try? parseInt("")
    let message = switch maybe {
        .some(v) => "parsed \(v)",
        .none => "missing",
    }
    let okLabel = switch ok {
        .some(v) => "\(v)",
        .none => "failed",
    }

    print("ok=\(okLabel) soft=\(soft) \(message)")
}
