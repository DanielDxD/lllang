#[test]
fn multi_file_tools() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().parent().unwrap()
        .join("examples/llm-agent");
    let tools_path = root.join("src/tools.ll");
    let main_path = root.join("src/main.ll");
    let tools = std::fs::read_to_string(&tools_path).unwrap();
    let main = std::fs::read_to_string(&main_path).unwrap();
    eprintln!("paths {} {}", tools_path.display(), main_path.display());
    let r = lllang_sema::analyze(&[
        (tools_path.display().to_string(), tools),
        (main_path.display().to_string(), main),
    ]);
    for d in &r.diagnostics {
        if d.severity == lllang_syntax::Severity::Error {
            eprintln!("{}: {}", d.code, d.message);
        }
    }
    let names: Vec<_> = r.module.functions.iter().map(|f| f.name.clone()).collect();
    eprintln!("funcs={names:?}");
    let errs: Vec<_> = r.diagnostics.iter().filter(|d| d.severity == lllang_syntax::Severity::Error).collect();
    assert!(errs.is_empty(), "{errs:?}");
}
