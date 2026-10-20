use lllang_mir::{emit_mir, lower};
use lllang_sema::analyze;

#[test]
fn lower_hello_has_ll_main() {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/hello/src/main.ll"),
    )
    .unwrap();
    let sema = analyze(&[("main.ll".into(), src)]);
    let mir = lower(&sema.module);
    assert!(mir.functions.iter().any(|f| f.name == "ll_main"));
    let text = emit_mir(&mir);
    assert!(text.contains("ll_main"));
}
