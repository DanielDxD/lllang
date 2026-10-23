//! Phase 7: `lang new` templates must check / build / run / test / bench.
//! Skipped when LLVM 22 is unavailable.

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
    let root = repo_root();
    [
        root.join("target/debug/lang"),
        root.join("target/release/lang"),
    ]
    .into_iter()
    .find(|c| c.exists())
    .unwrap_or_else(|| root.join("target/debug/lang"))
}

fn lang_cmd(dir: &Path) -> Command {
    let mut cmd = Command::new(lang_bin());
    cmd.current_dir(dir);
    if let Ok(prefix) = std::env::var("LLVM_SYS_221_PREFIX") {
        cmd.env("LLVM_SYS_221_PREFIX", prefix);
    } else {
        cmd.env("LLVM_SYS_221_PREFIX", "/opt/homebrew/opt/llvm@22");
    }
    cmd
}

fn assert_ok(label: &str, out: std::process::Output) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "{label} failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

fn scaffold(template: &str, name: &str) -> PathBuf {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Keep tempdir alive by leaking into a persistent path under target.
    let dest = repo_root().join("target/tmp-templates").join(name);
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    let out = lang_cmd(&repo_root())
        .args([
            "new",
            template,
            "-n",
            name,
            "--no-git",
            "-o",
            dest.to_str().unwrap(),
        ])
        .output()
        .expect("lang new");
    assert_ok("lang new", out);
    let _ = tmp; // silence
    dest
}

#[test]
fn template_console() {
    if !llvm_available() || !lang_bin().exists() {
        eprintln!("skip: LLVM/lang missing");
        return;
    }
    let dir = scaffold("console", "t-console");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("run", lang_cmd(&dir).args(["run"]).output().unwrap());
}

#[test]
fn template_lib() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("lib", "t-lib");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("test", lang_cmd(&dir).args(["test"]).output().unwrap());
}

#[test]
fn template_test() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("test", "t-testpkg");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("test", lang_cmd(&dir).args(["test"]).output().unwrap());
}

#[test]
fn template_cli() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("cli", "t-cli");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("test", lang_cmd(&dir).args(["test"]).output().unwrap());
    assert_ok("run", lang_cmd(&dir).args(["run"]).output().unwrap());
}

#[test]
fn template_web() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("web", "t-web");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("test", lang_cmd(&dir).args(["test"]).output().unwrap());
    assert_ok("run", lang_cmd(&dir).args(["run"]).output().unwrap());
}

#[test]
fn template_agent() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("agent", "t-agent");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("run", lang_cmd(&dir).args(["run"]).output().unwrap());
}

#[test]
fn template_mcp() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("mcp", "t-mcp");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("run", lang_cmd(&dir).args(["run"]).output().unwrap());
}

#[test]
fn template_bench() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("bench", "t-bench");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("test", lang_cmd(&dir).args(["test"]).output().unwrap());
    let out = lang_cmd(&dir).args(["bench"]).output().unwrap();
    assert_ok("bench", out.clone());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ms") || stdout.contains("bench"),
        "expected timed bench output, got:\n{stdout}"
    );
}

#[test]
fn template_solution() {
    if !llvm_available() || !lang_bin().exists() {
        return;
    }
    let dir = scaffold("solution", "t-solution");
    assert_ok("check", lang_cmd(&dir).args(["check"]).output().unwrap());
    assert_ok("run", lang_cmd(&dir).args(["run"]).output().unwrap());
}
