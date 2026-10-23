use lllang_sema::analyze;
use lllang_syntax::{parse, Decl, MemberDecl};

#[test]
fn computed_getter_in_hir() {
    let src = r#"
namespace T
public struct Point(x: Int) {
    public var doubled: Int => self.x
}
public func main() {
    let p = Point(x: 3)
    let d = p.doubled
}
"#;
    let pr = parse(src);
    let Decl::Struct(s) = pr.file.decls.iter().find(|d| matches!(d, Decl::Struct(_))).unwrap() else {
        panic!("no struct");
    };
    let vars: Vec<_> = s
        .members
        .iter()
        .filter_map(|m| match m {
            MemberDecl::Var(v) => Some(v.computed.is_some()),
            _ => None,
        })
        .collect();
    eprintln!("struct vars computed flags: {vars:?}");
    assert_eq!(vars, vec![true]);

    let r = analyze(&[("t.ll".into(), src.into())]);
    for d in &r.diagnostics {
        eprintln!("{:?}: {}", d.severity, d.message);
    }
    let names: Vec<_> = r.module.functions.iter().map(|f| f.name.clone()).collect();
    eprintln!("funcs: {names:?}");
    assert!(
        names.iter().any(|n| n.contains("__get_doubled") || n.contains("get_doubled")),
        "missing getter in {names:?}"
    );
}
