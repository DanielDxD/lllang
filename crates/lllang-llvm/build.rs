fn main() {
    let llvm = std::env::var("LLVM_SYS_221_PREFIX")
        .unwrap_or_else(|_| "/opt/homebrew/opt/llvm@22".into());
    if !std::path::Path::new(&llvm)
        .join("bin/llvm-config")
        .exists()
    {
        panic!(
            "LLVM 22 not found. brew install llvm@22 && export LLVM_SYS_221_PREFIX=/opt/homebrew/opt/llvm@22"
        );
    }
    println!("cargo:rustc-env=LLVM_SYS_221_PREFIX={llvm}");
    cc::Build::new()
        .file("../../runtime/lllang_rt.c")
        .std("c11")
        .compile("lllang_rt");
    println!("cargo:rerun-if-changed=../../runtime/lllang_rt.c");
    println!("cargo:rerun-if-changed=../../runtime/lllang_rt.h");
}
