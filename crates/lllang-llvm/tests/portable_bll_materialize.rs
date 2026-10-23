//! Portable bitcode emit + materialize for official triples (no link).

use lllang_llvm::{codegen_bitcode, materialize_bitcode, CodegenOptions};
use lllang_mir::lower;
use lllang_sema::analyze;
use std::path::PathBuf;

fn llvm_available() -> bool {
    std::path::Path::new("/opt/homebrew/opt/llvm@22/bin/llvm-config").exists()
        || std::env::var("LLVM_SYS_221_PREFIX")
            .map(|p| std::path::Path::new(&p).join("bin/llvm-config").exists())
            .unwrap_or(false)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn emit_bitcode_and_materialize_official_triples() {
    if !llvm_available() {
        eprintln!("skip: LLVM 22 not found");
        return;
    }

    let src = std::fs::read_to_string(repo_root().join("examples/hello/src/main.ll")).unwrap();
    let sema = analyze(&[("main.ll".into(), src)]);
    assert!(sema
        .diagnostics
        .iter()
        .all(|d| d.severity != lllang_syntax::Severity::Error));
    let mir = lower(&sema.module);

    let opts = CodegenOptions {
        optimize: false,
        emit_llvm_ir: false,
        target_triple: "aarch64-apple-darwin".into(),
    };
    let (_ir, bitcode) = codegen_bitcode(&mir, &opts).expect("bitcode");
    assert!(!bitcode.is_empty());

    let out = std::env::temp_dir().join("lllang_bll_materialize");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();

    let triples = [
        "aarch64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-pc-windows-msvc",
        "aarch64-pc-windows-msvc",
    ];
    for t in triples {
        let ext = if t.contains("windows") { "obj" } else { "o" };
        let obj = out.join(format!("{t}.{ext}"));
        materialize_bitcode(&bitcode, t, &obj, false)
            .unwrap_or_else(|e| panic!("materialize {t}: {e:#}"));
        assert!(obj.exists() && obj.metadata().unwrap().len() > 0, "{t}");
    }
}
