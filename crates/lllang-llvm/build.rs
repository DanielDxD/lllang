use std::env;
use std::path::PathBuf;

fn main() {
    let llvm = env::var("LLVM_SYS_221_PREFIX").unwrap_or_else(|_| {
        if cfg!(windows) {
            // Common winget / official LLVM install locations on Windows.
            r"C:\Program Files\LLVM".into()
        } else if cfg!(target_os = "linux") {
            "/usr/lib/llvm-22".into()
        } else {
            "/opt/homebrew/opt/llvm@22".into()
        }
    });
    let llvm_config = PathBuf::from(&llvm).join("bin").join(if cfg!(windows) {
        "llvm-config.exe"
    } else {
        "llvm-config"
    });
    if !llvm_config.exists() {
        panic!(
            "LLVM 22 not found at {llvm}. Install LLVM 22 and set LLVM_SYS_221_PREFIX \
             (macOS: brew install llvm@22; Windows: winget install LLVM.LLVM)."
        );
    }
    println!("cargo:rustc-env=LLVM_SYS_221_PREFIX={llvm}");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let runtime_dir = manifest_dir.join("../../runtime");
    let windows_sqlite = runtime_dir.join("windows");

    let mut build = cc::Build::new();
    build
        .file(runtime_dir.join("lllang_rt.c"))
        .include(&runtime_dir)
        .std("c11")
        .warnings(false);

    if cfg!(windows) {
        // Prefer clang so C11 `_Atomic` works; MSVC cl.exe is incomplete for C11 atomics.
        if which("clang").is_some() {
            build.compiler("clang");
        }
        build.define("_CRT_SECURE_NO_WARNINGS", None);
        build.define("WIN32_LEAN_AND_MEAN", None);
        let include = windows_sqlite.join("include");
        if include.join("sqlite3.h").exists() {
            build.include(&include);
        } else {
            panic!(
                "Windows SQLite headers missing. Run scripts/fetch-sqlite-windows.ps1 \
                 (or see runtime/windows/README.md)."
            );
        }
        let lib_dir = windows_sqlite.join("lib");
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=sqlite3");
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rerun-if-changed={}", windows_sqlite.join("include/sqlite3.h").display());
    } else {
        build.flag("-pthread");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=sqlite3");
    }

    build.compile("lllang_rt");

    // Expose the archive name for pack-release / runtime_lib_path resolution.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let archive_name = if cfg!(windows) {
        "lllang_rt.lib"
    } else {
        "liblllang_rt.a"
    };
    println!("cargo:rustc-env=LLLANG_RT_ARCHIVE={archive_name}");
    println!(
        "cargo:rustc-env=LLLANG_RT_OUT_DIR={}",
        out_dir.display()
    );

    println!("cargo:rerun-if-changed={}", runtime_dir.join("lllang_rt.c").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("lllang_rt.h").display());
    println!("cargo:rerun-if-changed={}", runtime_dir.join("ll_compat.h").display());
}

fn which(cmd: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let p = dir.join(cmd);
            if p.is_file() {
                Some(p)
            } else {
                let with_exe = dir.join(format!("{cmd}.exe"));
                if with_exe.is_file() {
                    Some(with_exe)
                } else {
                    None
                }
            }
        })
    })
}
