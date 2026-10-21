//! Smoke: `lang test` on examples/unit-tests (LLVM 22 required).

use std::path::{Path, PathBuf};
use std::process::Command;

fn llvm_available() -> bool {
    std::env::var("LLVM_SYS_221_PREFIX")
        .map(|p| Path::new(&p).join("bin/llvm-config").exists())
        .unwrap_or(false)
        || Path::new("/opt/homebrew/opt/llvm@22/bin/llvm-config").exists()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn lang_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    let candidates = [
        p.join("target/debug/lang"),
        p.join("target/release/lang"),
    ];
    candidates
        .into_iter()
        .find(|c| c.exists())
        .unwrap_or_else(|| p.join("target/debug/lang"))
}

#[test]
fn unit_tests_example() {
    if !llvm_available() {
        eprintln!("skip: LLVM 22 not found");
        return;
    }
    let lang = lang_bin();
    if !lang.exists() {
        eprintln!("skip: lang binary not found");
        return;
    }
    let root = repo_root();
    let manifest = root.join("examples/unit-tests/Package.ll");
    let mut cmd = Command::new(&lang);
    cmd.args(["test", "--manifest", manifest.to_str().unwrap()]);
    if let Ok(prefix) = std::env::var("LLVM_SYS_221_PREFIX") {
        cmd.env("LLVM_SYS_221_PREFIX", prefix);
    } else {
        cmd.env("LLVM_SYS_221_PREFIX", "/opt/homebrew/opt/llvm@22");
    }
    if let Ok(sdk) = std::env::var("SDKROOT") {
        cmd.env("SDKROOT", sdk);
    }
    let output = cmd.output().expect("run lang test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "lang test failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("passed") || stderr.contains("passed") || stdout.contains("✓"),
        "expected pass summary\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
