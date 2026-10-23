//! LLVM backend for LLLang (Phase 2).

mod codegen;

use anyhow::{Context, Result};
use codegen::{emit_bitcode as emit_bitcode_inner, emit_object, materialize_bitcode_to_object};
use lllang_mir::Program;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CodegenOptions {
    pub optimize: bool,
    pub emit_llvm_ir: bool,
    /// Official LLVM target triple (e.g. `aarch64-apple-darwin`).
    pub target_triple: String,
}

pub struct CodegenResult {
    pub object_path: PathBuf,
    pub llvm_ir: Option<String>,
}

/// Resolve the C runtime archive for `triple`.
///
/// Order: `LANG_RUNTIME_LIB` (only when targeting the host) →
/// `$LANG_HOME/toolchain/lib/<triple>/<archive>` →
/// `$LANG_HOME/sdk/<triple>/lib/<archive>` →
/// next to `lang` exe `lib/<triple>/` →
/// for **host triple only**: legacy flat `lib/<archive>` and compile-time `OUT_DIR`.
pub fn runtime_lib_path_for(triple: &str) -> PathBuf {
    let archive = if triple.contains("windows") {
        "lllang_rt.lib"
    } else {
        "liblllang_rt.a"
    };
    let host = host_triple_guess();
    let is_host = triple == host;

    if is_host {
        if let Ok(p) = std::env::var("LANG_RUNTIME_LIB") {
            return PathBuf::from(p);
        }
    }

    let per_triple = |home: PathBuf| -> Vec<PathBuf> {
        vec![
            home.join("toolchain").join("lib").join(triple).join(archive),
            home.join("sdk").join(triple).join("lib").join(archive),
        ]
    };
    if let Ok(home) = std::env::var("LANG_HOME") {
        for p in per_triple(PathBuf::from(&home)) {
            if p.exists() {
                return p;
            }
        }
        if is_host {
            let flat = PathBuf::from(&home).join("toolchain").join("lib").join(archive);
            if flat.exists() {
                return flat;
            }
        }
    }
    if let Some(home) = dirs_home() {
        let root = home.join(".lang");
        for p in per_triple(root.clone()) {
            if p.exists() {
                return p;
            }
        }
        if is_host {
            let flat = root.join("toolchain").join("lib").join(archive);
            if flat.exists() {
                return flat;
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin) = exe.parent() {
            let per = bin.join("..").join("lib").join(triple).join(archive);
            if let Ok(canon) = per.canonicalize() {
                return canon;
            }
            if per.exists() {
                return per;
            }
            if is_host {
                let flat = bin.join("..").join("lib").join(archive);
                if let Ok(canon) = flat.canonicalize() {
                    return canon;
                }
                if flat.exists() {
                    return flat;
                }
            }
        }
    }
    if is_host {
        return PathBuf::from(env!("OUT_DIR")).join(archive);
    }
    // Preferred path for diagnostics when missing.
    dirs_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".lang")
        .join("toolchain")
        .join("lib")
        .join(triple)
        .join(archive)
}

pub fn runtime_lib_path() -> PathBuf {
    runtime_lib_path_for(&host_triple_guess())
}

fn host_triple_guess() -> String {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => "aarch64-apple-darwin".into(),
        ("x86_64", "macos") => "x86_64-apple-darwin".into(),
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu".into(),
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu".into(),
        ("x86_64", "windows") => "x86_64-pc-windows-msvc".into(),
        ("aarch64", "windows") => "aarch64-pc-windows-msvc".into(),
        (a, o) => format!("{a}-unknown-{o}"),
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Directory containing Windows sqlite3 import lib / DLL (optional).
pub fn windows_sqlite_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("LANG_SQLITE_DIR") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    if let Ok(home) = std::env::var("LANG_HOME") {
        let p = PathBuf::from(home).join("toolchain").join("runtime").join("windows");
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs_home() {
        let p = home
            .join(".lang")
            .join("toolchain")
            .join("runtime")
            .join("windows");
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bin) = exe.parent() {
            let p = bin.join("..").join("runtime").join("windows");
            if p.exists() {
                return Some(p);
            }
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let p = manifest.join("../../runtime/windows");
    if p.exists() {
        return Some(p);
    }
    None
}

pub fn codegen(program: &Program, out_dir: &Path, opts: &CodegenOptions) -> Result<CodegenResult> {
    std::fs::create_dir_all(out_dir).context("create out_dir")?;
    let ext = if opts.target_triple.contains("windows") {
        "obj"
    } else {
        "o"
    };
    let object_path = out_dir.join(format!("out.{ext}"));
    let llvm_ir = emit_object(program, &object_path, opts)?.0;
    Ok(CodegenResult {
        object_path,
        llvm_ir: if opts.emit_llvm_ir { Some(llvm_ir) } else { None },
    })
}

/// Emit portable LLVM bitcode for BLL v2 packing.
pub fn codegen_bitcode(program: &Program, opts: &CodegenOptions) -> Result<(Option<String>, Vec<u8>)> {
    let (ir, bc) = emit_bitcode_inner(program, opts)?;
    Ok((if opts.emit_llvm_ir { Some(ir) } else { None }, bc))
}

/// Materialize BLL v2 bitcode payload to a native object for `triple`.
pub fn materialize_bitcode(
    bitcode: &[u8],
    triple: &str,
    object_path: &Path,
    optimize: bool,
) -> Result<()> {
    materialize_bitcode_to_object(bitcode, triple, object_path, optimize)
}

pub fn link_executable(
    objects: &[PathBuf],
    output: &Path,
    _optimize: bool,
    target_triple: &str,
) -> Result<()> {
    let clang = which_clang_for(target_triple);
    let rt = runtime_lib_path_for(target_triple);
    if !rt.exists() {
        anyhow::bail!(
            "runtime archive not found for `{target_triple}` at {}. \
             Set LANG_RUNTIME_LIB or install under $LANG_HOME/toolchain/lib/{target_triple}/ \
             (see `lang sdk install {target_triple}`).",
            rt.display()
        );
    }
    let mut cmd = Command::new(&clang);
    cmd.arg(format!("--target={target_triple}"));
    if target_triple.contains("apple-darwin") {
        if let Some(sdk) = macos_sdk_path() {
            cmd.arg("-isysroot").arg(sdk);
        }
    }
    // Optional sysroot for cross Linux/Windows from LANG_HOME/sdk/<triple>.
    let sysroot = lang_home()
        .join("sdk")
        .join(target_triple)
        .join("sysroot");
    if sysroot.is_dir() {
        cmd.arg(format!("--sysroot={}", sysroot.display()));
    }
    cmd.arg("-o").arg(output);
    for obj in objects {
        cmd.arg(obj);
    }
    cmd.arg(&rt);

    if target_triple.contains("windows") {
        if let Some(sqlite_root) = windows_sqlite_dir() {
            let lib = sqlite_root.join("lib");
            cmd.arg(format!("-L{}", lib.display()));
            let dll = sqlite_root.join("bin").join("sqlite3.dll");
            if dll.exists() {
                if let Some(out_dir) = output.parent() {
                    let dest = out_dir.join("sqlite3.dll");
                    let _ = std::fs::copy(&dll, &dest);
                }
            }
        }
        cmd.arg("-lsqlite3");
        cmd.arg("-lws2_32");
        cmd.arg("-ladvapi32");
    } else {
        cmd.arg("-pthread");
        cmd.arg("-lsqlite3");
        if target_triple.contains("apple-darwin") {
            cmd.arg("-lSystem");
        }
    }

    let status = cmd.status().context("link")?;
    if !status.success() {
        anyhow::bail!("link failed: {cmd:?}");
    }
    Ok(())
}

fn lang_home() -> PathBuf {
    if let Ok(h) = std::env::var("LANG_HOME") {
        return PathBuf::from(h);
    }
    dirs_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".lang")
}

fn which_clang_for(target_triple: &str) -> String {
    // Prefer LLVM's clang for non-Darwin cross targets (Apple clang cannot drive GNU ld flags).
    let want_llvm = !target_triple.contains("apple-darwin");
    if want_llvm {
        if let Ok(prefix) = std::env::var("LLVM_SYS_221_PREFIX") {
            let p = PathBuf::from(prefix).join("bin").join("clang");
            if p.exists() {
                return p.display().to_string();
            }
        }
        let brew = PathBuf::from("/opt/homebrew/opt/llvm@22/bin/clang");
        if brew.exists() {
            return brew.display().to_string();
        }
    }
    if target_triple.contains("apple-darwin") {
        if let Ok(out) = Command::new("xcrun").args(["-f", "clang"]).output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return p;
                }
            }
        }
    }
    if Command::new("clang").arg("--version").output().is_ok() {
        "clang".into()
    } else if cfg!(windows) {
        r"C:\Program Files\LLVM\bin\clang.exe".into()
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
