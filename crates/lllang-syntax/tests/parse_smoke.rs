use lllang_syntax::{parse, check, Decl};

const HELLO: &str = r#"namespace Hello

import Std.IO

public func main() {
    print("Hello, LLLang!")
}
"#;

#[test]
fn parses_hello_world() {
    let result = parse(HELLO);
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    assert_eq!(result.file.decls.len(), 3);
    assert!(matches!(result.file.decls[0], Decl::Namespace(_)));
    assert!(matches!(result.file.decls[1], Decl::Import(_)));
    assert!(matches!(result.file.decls[2], Decl::Func(_)));
}

#[test]
fn check_hello_is_clean() {
    assert!(check(HELLO).is_empty());
}

#[test]
fn guard_statement() {
    let s = r#"func f() { guard let d = delegate else { return } }"#;
    assert!(check(s).is_empty(), "{:?}", check(s));
}

#[test]
fn parses_richer_syntax() {
    let src = r#"
namespace Demo

import Lib.*

@MainActor
public struct Point(public let x: Int, public let y: Int) {
    func distance() -> Double => (x * x + y * y) as Double
}

public enum Status {
    ok,
    err(String),
}

public class Worker(name: String): Runnable {
    weak var delegate: Delegate?
    async func run() throws(IOError) -> Result {
        guard let d = delegate else { return Status.err("no delegate") }
        switch d.kind {
            .ready => "go"
            default => "wait"
        }
    }
}

public protocol Runnable { }

extension Array where Element: Equatable {
    func contains(_ item: Element) -> Bool {
        for e in self where e == item {
            return true
        }
        return false
    }
}

func demo() {
    let msg = "Hello, \(name)!"
    let n = try? load()
    do {
        try work()
    } catch e => print(e)
}
"#;
    let result = parse(src);
    assert!(
        result.diagnostics.is_empty(),
        "diagnostics: {:?}",
        result.diagnostics
    );
    assert!(result.file.decls.len() >= 5);
}
