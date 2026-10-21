use lllang_syntax::{parse, Decl, FuncBody, Severity, Stmt};

#[test]
fn sequential_if_stmts_in_block() {
    let src = r#"
namespace T
public func main() {
    let job = 4
    if job == 0 { print("0") }
    if job == 1 { print("1") }
    if job == 4 { print("4") }
}
"#;
    let result = parse(src);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity != Severity::Error),
        "{:?}",
        result.diagnostics
    );
    let Decl::Func(f) = result
        .file
        .decls
        .iter()
        .find(|d| matches!(d, Decl::Func(_)))
        .unwrap()
    else {
        panic!()
    };
    let FuncBody::Block(b) = &f.body else {
        panic!("no block")
    };
    let ifs = b
        .stmts
        .iter()
        .filter(|s| matches!(s, Stmt::If { .. }))
        .count();
    eprintln!("AST stmts={}, ifs={ifs}", b.stmts.len());
    for (i, s) in b.stmts.iter().enumerate() {
        eprintln!(
            "  stmt{i}: {}",
            match s {
                Stmt::Let(_) => "let",
                Stmt::If { .. } => "if",
                Stmt::Expr { .. } => "expr",
                _ => "other",
            }
        );
    }
    assert_eq!(ifs, 3);
}
