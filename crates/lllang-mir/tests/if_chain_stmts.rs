use lllang_mir::lower;
use lllang_sema::analyze;
use lllang_sema::hir::Stmt;

#[test]
fn sequential_ifs_all_present_in_mir() {
    let src = r#"
namespace T
public func job0() {}
public func job1() {}
public func job4() {}
public func main() {
    let job = 4
    if job == 0 { job0() }
    if job == 1 { job1() }
    if job == 4 { job4() }
}
"#;
    let sema = analyze(&[("t.ll".into(), src.into())]);
    let errs: Vec<_> = sema
        .diagnostics
        .iter()
        .filter(|d| format!("{:?}", d).contains("Error"))
        .collect();
    assert!(errs.is_empty(), "{:?}", sema.diagnostics);

    let main = sema
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with(".main") || f.name == "main" || f.name == "T.main")
        .expect("main");
    let if_count = main
        .body
        .as_ref()
        .unwrap()
        .stmts
        .iter()
        .filter(|s| matches!(s, Stmt::If { .. }))
        .count();
    eprintln!(
        "HIR if count = {if_count}, stmts = {}",
        main.body.as_ref().unwrap().stmts.len()
    );
    assert_eq!(if_count, 3, "expected 3 sequential if stmts in HIR");

    let mir = lower(&sema.module);
    let f = mir
        .functions
        .iter()
        .find(|f| f.name == "ll_main")
        .expect("ll_main");
    let branches = f
        .blocks
        .iter()
        .filter(|b| matches!(b.term, lllang_mir::Terminator::Branch { .. }))
        .count();
    eprintln!("MIR branch terminators = {branches}");
    assert!(branches >= 3, "expected >=3 branches, got {branches}");
}
