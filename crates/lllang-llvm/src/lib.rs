//! LLVM backend for LLLang (Phase 2).

mod codegen;

use anyhow::{Context, Result};
use codegen::emit_object;
use lllang_mir::Program;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CodegenOptions {
    pub optimize: bool,
    pub emit_llvm_ir: bool,
}

pub struct CodegenResult {
    pub object_path: PathBuf,
    pub llvm_ir: Option<String>,
}

pub fn runtime_lib_path() -> PathBuf {
    PathBuf::from(env!("OUT_DIR")).join("liblllang_rt.a")
}

pub fn codegen(program: &Program, out_dir: &Path, opts: &CodegenOptions) -> Result<CodegenResult> {
    std::fs::create_dir_all(out_dir).context("create out_dir")?;
    let object_path = out_dir.join("out.o");
    let llvm_ir = emit_object(program, &object_path, opts)?.0;
    Ok(CodegenResult {
        object_path,
        llvm_ir: if opts.emit_llvm_ir { Some(llvm_ir) } else { None },
    })
}

pub fn link_executable(objects: &[PathBuf], output: &Path, _optimize: bool) -> Result<()> {
    let clang = which_clang();
    let rt = runtime_lib_path();
    let mut cmd = Command::new(&clang);
    if let Some(sdk) = macos_sdk_path() {
        cmd.arg("-isysroot").arg(sdk);
    }
    cmd.arg("-o").arg(output);
    for obj in objects {
        cmd.arg(obj);
    }
    cmd.arg(&rt);
    // Apple platforms need libSystem via the SDK; also link libc++/libSystem explicitly on Darwin.
    if cfg!(target_os = "macos") {
        cmd.arg("-lSystem");
    }
    let status = cmd.status().context("link")?;
    if !status.success() {
        anyhow::bail!("link failed: {cmd:?}");
    }
    Ok(())
}

fn which_clang() -> String {
    // Prefer Apple clang for host linking (correct SDK); LLVM's clang is for IR codegen only.
    if let Ok(out) = Command::new("xcrun").args(["-f", "clang"]).output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return p;
            }
        }
    }
    if Command::new("clang").arg("--version").output().is_ok() {
        "clang".into()
    } else {
        "/opt/homebrew/opt/llvm@22/bin/clang".into()
    }
}

fn macos_sdk_path() -> Option<PathBuf> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    if let Ok(out) = Command::new("xcrun").args(["--show-sdk-path"]).output() {
        if out.status.success() {
            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !p.is_empty() {
                return Some(PathBuf::from(p));
            }
        }
    }
    None
}
