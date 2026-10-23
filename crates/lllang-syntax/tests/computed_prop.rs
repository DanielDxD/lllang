use lllang_syntax::{parse, Decl, MemberDecl, ComputedBody};

#[test]
fn parses_computed_arrow_and_get() {
    let src = r#"
namespace T
public struct Point(x: Int) {
    public var doubled: Int => self.x
    public var six: Int {
        get { return 6 }
    }
}
"#;
    let r = parse(src);
    assert!(r.diagnostics.iter().all(|d| d.severity != lllang_syntax::Severity::Error), "{:?}", r.diagnostics);
    let Decl::Struct(s) = r.file.decls.iter().find(|d| matches!(d, Decl::Struct(_))).unwrap() else { panic!() };
    let mut computed = 0;
    for m in &s.members {
        if let MemberDecl::Var(v) = m {
            eprintln!("var computed={:?}", v.computed.is_some());
            if v.computed.is_some() { computed += 1; }
            if let Some(ComputedBody::Arrow(_)) = &v.computed {
                eprintln!("arrow ok");
            }
            if let Some(ComputedBody::Accessors { .. }) = &v.computed {
                eprintln!("accessors ok");
            }
        }
    }
    assert_eq!(computed, 2, "expected 2 computed vars");
}
