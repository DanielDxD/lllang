//! End-to-end: compile and run Phase-2 example packages against expected.txt.
//! Skipped automatically when LLVM 22 is not installed.

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
    // Prefer the binary built for this test package.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates
    p.pop(); // repo
    let candidates = [
        p.join("target/debug/lang"),
        p.join("target/release/lang"),
    ];
    candidates
        .into_iter()
        .find(|c| c.exists())
        .unwrap_or_else(|| p.join("target/debug/lang"))
}

fn run_example(args: &[&str], expected_path: &Path) {
    if !llvm_available() {
        eprintln!("skip: LLVM 22 not found (set LLVM_SYS_221_PREFIX)");
        return;
    }
    let lang = lang_bin();
    if !lang.exists() {
        eprintln!("skip: lang binary not found at {}", lang.display());
        return;
    }
    let mut cmd = Command::new(&lang);
    cmd.args(args);
    if let Ok(prefix) = std::env::var("LLVM_SYS_221_PREFIX") {
        cmd.env("LLVM_SYS_221_PREFIX", prefix);
    } else {
        cmd.env("LLVM_SYS_221_PREFIX", "/opt/homebrew/opt/llvm@22");
    }
    if let Ok(sdk) = std::env::var("SDKROOT") {
        cmd.env("SDKROOT", sdk);
    }
    let output = cmd.output().expect("run lang");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "lang {:?} failed\nstdout:\n{stdout}\nstderr:\n{stderr}",
        args
    );
    let expected = std::fs::read_to_string(expected_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", expected_path.display()));
    // lang run prints build chatter on stdout before program output; compare trailing lines.
    let out_trim = stdout.trim_end();
    let exp_trim = expected.trim_end();
    assert!(
        out_trim.ends_with(exp_trim),
        "stdout did not end with expected.txt\n--- stdout ---\n{out_trim}\n--- expected ---\n{exp_trim}\n--- stderr ---\n{stderr}"
    );
}

#[test]
fn hello() {
    let root = repo_root();
    run_example(
        &[
            "run",
            "--manifest",
            root.join("examples/hello/Package.ll").to_str().unwrap(),
        ],
        &root.join("examples/hello/expected.txt"),
    );
}

#[test]
fn errors() {
    let root = repo_root();
    run_example(
        &[
            "run",
            "--manifest",
            root.join("examples/errors/Package.ll").to_str().unwrap(),
        ],
        &root.join("examples/errors/expected.txt"),
    );
}

#[test]
fn shapes_oop() {
    let root = repo_root();
    run_example(
        &[
            "run",
            "--manifest",
            root.join("examples/shapes-oop/Package.ll").to_str().unwrap(),
        ],
        &root.join("examples/shapes-oop/expected.txt"),
    );
}

#[test]
fn shop_solution() {
    let root = repo_root();
    run_example(
        &[
            "run",
            "--solution",
            root.join("examples/shop-solution/Solution.ll")
                .to_str()
                .unwrap(),
            "--project",
            "Cli",
        ],
        &root.join("examples/shop-solution/expected.txt"),
    );
}

#[test]
fn json_toon() {
    let root = repo_root();
    run_example(
        &[
            "run",
            "--manifest",
            root.join("examples/json-toon/Package.ll").to_str().unwrap(),
        ],
        &root.join("examples/json-toon/expected.txt"),
    );
}

#[test]
fn async_http() {
    let root = repo_root();
    run_example(
        &[
            "run",
            "--manifest",
            root.join("examples/async-http/Package.ll").to_str().unwrap(),
        ],
        &root.join("examples/async-http/expected.txt"),
    );
}

#[test]
fn llm_agent() {
    let root = repo_root();
    if !llvm_available() {
        return;
    }
    let lang = lang_bin();
    if !lang.exists() {
        return;
    }
    let mut cmd = Command::new(&lang);
    cmd.args([
        "run",
        "--manifest",
        root.join("examples/llm-agent/Package.ll").to_str().unwrap(),
    ]);
    cmd.env("LANG_LLM_MOCK", "summary: llm-agent");
    if let Ok(prefix) = std::env::var("LLVM_SYS_221_PREFIX") {
        cmd.env("LLVM_SYS_221_PREFIX", prefix);
    } else {
        cmd.env("LLVM_SYS_221_PREFIX", "/opt/homebrew/opt/llvm@22");
    }
    let output = cmd.output().expect("run lang");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "llm-agent failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let expected = std::fs::read_to_string(root.join("examples/llm-agent/expected.txt")).unwrap();
    assert!(
        stdout.trim_end().ends_with(expected.trim_end()),
        "stdout={stdout}\nexpected={expected}"
    );
}
