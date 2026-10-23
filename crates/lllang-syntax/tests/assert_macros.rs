use lllang_syntax::{parse, Severity};

fn assert_ok(src: &str, label: &str) {
    let r = parse(src);
    let errs: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    assert!(errs.is_empty(), "{label}: {errs:?}\n{src}");
}

#[test]
fn assertion_macros_in_body() {
    assert_ok(
        r#"
namespace N
public func f() {
    let result = 5 - 25
    @expect(result < 0)
    @lt(result, 0)
}
"#,
        "cmp",
    );
    assert_ok(
        r#"
namespace N
public struct S {
    @test("t")
    public func testNeg() {
        let result = 5 - 25
        @expect(result < 0)
        @lt(result, 0)
    }

    @test("string emptiness")
    public func testEmpty() {
        @empty("")
        @notEmpty("x")
        @contains("hello", "ell")
    }
}
"#,
        "suite",
    );
}
