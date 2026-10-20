use lllang_llvm::{codegen, link_executable, CodegenOptions};
use lllang_mir::lower;
use lllang_sema::analyze;
use std::path::PathBuf;
use std::process::Command;

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
fn compile_and_run_hello() {
    if !llvm_available() {
        eprintln!("skip compile_and_run_hello: LLVM 22 not found");
        return;
    }

    let src = std::fs::read_to_string(repo_root().join("examples/hello/src/main.ll")).unwrap();
    let sema = analyze(&[("main.ll".into(), src)]);
    assert!(
        sema.diagnostics.iter().all(|d| d.severity != lllang_syntax::Severity::Error),
        "{:?}",
        sema.diagnostics
    );

    let mir = lower(&sema.module);
    let out_dir = std::env::temp_dir().join("lllang_hello_test");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();

    let opts = CodegenOptions {
        optimize: false,
        emit_llvm_ir: false,
    };
    let gen = codegen(&mir, &out_dir, &opts).expect("codegen");
    let bin = out_dir.join("hello");
    link_executable(&[gen.object_path], &bin, false).expect("link");

    let output = Command::new(&bin).output().expect("run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello"),
        "stdout: {stdout}, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
