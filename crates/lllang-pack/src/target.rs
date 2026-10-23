//! Official multi-target triples and Go-like aliases (`linux/amd64`, …).

use anyhow::{bail, Result};

/// Canonical LLVM triples supported for `lang build --target` / pack finalize.
pub const OFFICIAL_TRIPLES: &[&str] = &[
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
];

/// Lockfile / META marker for a portable (non-native) `.bll`.
pub const PORTABLE_TRIPLE: &str = "portable";

/// Current BLL container format version.
pub const BLL_FORMAT_VERSION: u16 = 2;

/// Language ABI epoch embedded in BLL v2 headers.
pub const BLL_ABI_EPOCH: u16 = 1;

pub fn is_official_triple(triple: &str) -> bool {
    OFFICIAL_TRIPLES.contains(&triple)
}

/// Map Go-like alias or full triple → official LLVM triple.
pub fn resolve_target_spec(spec: &str) -> Result<String> {
    let s = spec.trim();
    if s.is_empty() {
        bail!("empty --target");
    }
    // Reject known-unsupported aliases early with a clear message.
    let lower = s.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "darwin/amd64"
            | "darwin/x86_64"
            | "macos/amd64"
            | "macos/x86_64"
            | "osx/amd64"
    ) {
        bail!("unsupported target `{s}`: macOS x86_64 is not an official LLLang target (arm64 only)");
    }
    if is_official_triple(s) {
        return Ok(s.to_string());
    }
    let triple = match lower.as_str() {
        "darwin/arm64" | "macos/arm64" | "osx/arm64" => "aarch64-apple-darwin",
        "linux/amd64" | "linux/x86_64" => "x86_64-unknown-linux-gnu",
        "linux/arm64" | "linux/aarch64" => "aarch64-unknown-linux-gnu",
        "windows/amd64" | "windows/x86_64" | "win/amd64" => "x86_64-pc-windows-msvc",
        "windows/arm64" | "windows/aarch64" | "win/arm64" => "aarch64-pc-windows-msvc",
        _ => bail!(
            "unsupported target `{s}`; official: darwin/arm64, linux/amd64, linux/arm64, windows/amd64, windows/arm64 (or full LLVM triples)"
        ),
    };
    Ok(triple.into())
}

/// Resolve build target: explicit `--target`, else `LANG_OS`+`LANG_ARCH`, else host if official.
pub fn resolve_build_target(cli_target: Option<&str>) -> Result<String> {
    if let Some(t) = cli_target {
        return resolve_target_spec(t);
    }
    if let (Ok(os), Ok(arch)) = (std::env::var("LANG_OS"), std::env::var("LANG_ARCH")) {
        let alias = format!(
            "{}/{}",
            normalize_os(&os)?,
            normalize_arch(&arch)?
        );
        return resolve_target_spec(&alias);
    }
    let host = crate::host_triple();
    if is_official_triple(&host) {
        return Ok(host);
    }
    bail!(
        "host triple `{host}` is not an official LLLang target; pass `--target` \
         (e.g. linux/amd64) or set LANG_OS + LANG_ARCH"
    )
}

fn normalize_os(os: &str) -> Result<&'static str> {
    match os.trim().to_ascii_lowercase().as_str() {
        "darwin" | "macos" | "osx" => Ok("darwin"),
        "linux" => Ok("linux"),
        "windows" | "win" => Ok("windows"),
        other => bail!("unsupported LANG_OS `{other}` (use darwin, linux, or windows)"),
    }
}

fn normalize_arch(arch: &str) -> Result<&'static str> {
    match arch.trim().to_ascii_lowercase().as_str() {
        "amd64" | "x86_64" | "x64" => Ok("amd64"),
        "arm64" | "aarch64" => Ok("arm64"),
        other => bail!("unsupported LANG_ARCH `{other}` (use amd64 or arm64)"),
    }
}

/// Object file extension for a triple.
pub fn object_ext(triple: &str) -> &'static str {
    if triple.contains("windows") {
        "obj"
    } else {
        "o"
    }
}

/// Runtime archive filename for a triple.
pub fn runtime_archive_name(triple: &str) -> &'static str {
    if triple.contains("windows") {
        "lllang_rt.lib"
    } else {
        "liblllang_rt.a"
    }
}

/// True when the triple is a Windows MSVC target.
pub fn is_windows_triple(triple: &str) -> bool {
    triple.contains("windows")
}

/// True when the triple is Apple Darwin.
pub fn is_darwin_triple(triple: &str) -> bool {
    triple.contains("apple-darwin")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases() {
        assert_eq!(
            resolve_target_spec("linux/amd64").unwrap(),
            "x86_64-unknown-linux-gnu"
        );
        assert_eq!(
            resolve_target_spec("macos/arm64").unwrap(),
            "aarch64-apple-darwin"
        );
        assert_eq!(
            resolve_target_spec("windows/arm64").unwrap(),
            "aarch64-pc-windows-msvc"
        );
    }

    #[test]
    fn reject_macos_x86() {
        assert!(resolve_target_spec("macos/amd64").is_err());
    }
}
