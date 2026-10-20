use lllang_sema::analyze;
use lllang_syntax::{Diagnostic, Severity};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_example(rel: &str) -> String {
    std::fs::read_to_string(repo_root().join(rel)).expect("read example source")
}

fn errors(result: &lllang_sema::SemaResult) -> Vec<&Diagnostic> {
    result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

#[test]
fn hello_main_zero_errors() {
    let src = read_example("examples/hello/src/main.ll");
    let result = analyze(&[("main.ll".into(), src)]);
    assert!(errors(&result).is_empty(), "{:?}", result.diagnostics);
    assert!(
        result.module.functions.iter().any(|f| f.name.ends_with("main")),
        "expected main in {:?}",
        result.module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn errors_demo_zero_errors() {
    let src = read_example("examples/errors/src/main.ll");
    let result = analyze(&[("main.ll".into(), src)]);
    assert!(errors(&result).is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn shapes_oop_zero_errors() {
    let src = read_example("examples/shapes-oop/src/main.ll");
    let result = analyze(&[("main.ll".into(), src)]);
    assert!(errors(&result).is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn type_mismatch_emits_ll2000() {
    let src = r#"
namespace Test

func demo() {
    let x: Int = "hi"
}
"#;
    let result = analyze(&[("main.ll".into(), src.to_string())]);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "LL2000" && d.severity == Severity::Error),
        "{:?}",
        result.diagnostics
    );
}
