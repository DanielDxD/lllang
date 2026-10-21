//! Pack `.sbl` / `.bll`, global store, and `Package.lock` (Phase 3 MVP).

use anyhow::{bail, Context, Result};
use lllang_syntax::{parse, Access, Decl, FuncDecl, MemberDecl};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub mod lock;
pub mod registry;
pub mod store;

pub use lock::{LockFile, LockedPackage, LockedTarget};
pub use registry::{registry_root, FileRegistry, RegistryPackage};
pub use store::{host_triple, lang_home, store_path, Store};

/// Emit a textual `.sbl` surface from public declarations in sources.
pub fn emit_sbl(package_name: &str, version: &str, sources: &[(String, String)]) -> Result<String> {
    let mut out = String::new();
    out.push_str("// sbl-format: 1\n");
    out.push_str(&format!("package {package_name}@{version}\n\n"));
    for (_path, src) in sources {
        let parsed = parse(src);
        if parsed
            .diagnostics
            .iter()
            .any(|d| d.severity == lllang_syntax::Severity::Error)
        {
            continue;
        }
        for d in &parsed.file.decls {
            if let Some(s) = decl_sbl(d) {
                out.push_str(&s);
                out.push('\n');
            }
        }
    }
    Ok(out)
}

fn is_public(access: Option<Access>) -> bool {
    matches!(access, Some(Access::Public) | None)
}

fn decl_sbl(d: &Decl) -> Option<String> {
    match d {
        Decl::Namespace(n) => Some(format!("namespace {}", n.path.segments.join("."))),
        Decl::Struct(s) if is_public(s.access) => {
            let mut line = format!("public struct {}", s.name);
            if !s.primary_ctor.is_empty() {
                let params: Vec<_> = s
                    .primary_ctor
                    .iter()
                    .map(|p| format!("{}: {}", p.name, type_approx(&p.ty)))
                    .collect();
                line.push('(');
                line.push_str(&params.join(", "));
                line.push(')');
            }
            if let Some(inh) = &s.inheritance {
                let items: Vec<_> = inh
                    .items
                    .iter()
                    .map(type_approx)
                    .collect();
                line.push_str(": ");
                line.push_str(&items.join(", "));
            }
            Some(line)
        }
        Decl::Class(c) if is_public(c.access) => {
            Some(format!("public class {}", c.name))
        }
        Decl::Enum(e) if is_public(e.access) => Some(format!("public enum {}", e.name)),
        Decl::Protocol(p) if is_public(p.access) => Some(format!("public protocol {}", p.name)),
        Decl::Func(f) if is_public(f.access) => Some(func_sig_sbl(f)),
        Decl::Struct(s) => {
            let mut members = String::new();
            for m in &s.members {
                if let MemberDecl::Func(f) = m {
                    if is_public(f.access) {
                        members.push_str("  ");
                        members.push_str(&func_sig_sbl(f));
                        members.push('\n');
                    }
                }
            }
            if members.is_empty() {
                None
            } else {
                Some(format!("extension {} {{\n{}}}", s.name, members))
            }
        }
        _ => None,
    }
}

fn func_sig_sbl(f: &FuncDecl) -> String {
    let params: Vec<_> = f
        .params
        .iter()
        .map(|p| format!("{}: {}", p.internal_name, type_approx(&p.ty)))
        .collect();
    let mut s = format!("public func {}({})", f.name, params.join(", "));
    if let Some(rt) = &f.return_type {
        s.push_str(" -> ");
        s.push_str(&type_approx(rt));
    }
    if !f.throws.types.is_empty() {
        let ts: Vec<_> = f.throws.types.iter().map(type_approx).collect();
        s.push_str(&format!(" throws({})", ts.join(", ")));
    }
    s
}

fn type_approx(t: &lllang_syntax::Type) -> String {
    match t {
        lllang_syntax::Type::Named { name, generics, .. } => {
            if generics.is_empty() {
                name.clone()
            } else {
                let g: Vec<_> = generics.iter().map(type_approx).collect();
                format!("{name}<{}>", g.join(", "))
            }
        }
        lllang_syntax::Type::Optional { inner, .. } => format!("{}?", type_approx(inner)),
        lllang_syntax::Type::Array { element, .. } => format!("[{}]", type_approx(element)),
        _ => "Any".into(),
    }
}

/// Content hash (sha256 hex) of bytes.
pub fn content_hash(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Pack a library: write `package.sbl` and `targets/<triple>.bll` (object bytes) under `out_dir`.
pub fn pack_library(
    package_name: &str,
    version: &str,
    sources: &[(String, String)],
    object_bytes: &[u8],
    out_dir: &Path,
) -> Result<(PathBuf, PathBuf, String, String)> {
    let sbl = emit_sbl(package_name, version, sources)?;
    let sbl_hash = content_hash(sbl.as_bytes());
    let bll_hash = content_hash(object_bytes);
    let triple = host_triple();
    fs::create_dir_all(out_dir.join("targets"))?;
    let sbl_path = out_dir.join("package.sbl");
    fs::write(&sbl_path, &sbl)?;
    let bll_path = out_dir.join("targets").join(format!("{triple}.bll"));
    fs::write(&bll_path, object_bytes)?;
    // Integrity footer
    let mut sbl_with_footer = sbl;
    sbl_with_footer.push_str(&format!(
        "\n// integrity: sbl={sbl_hash} bll={bll_hash} triple={triple}\n"
    ));
    fs::write(&sbl_path, &sbl_with_footer)?;
    Ok((sbl_path, bll_path, sbl_hash, bll_hash))
}

/// Install packed artifacts into the global store; return store entry path.
pub fn install_to_store(
    package_name: &str,
    version: &str,
    sbl_bytes: &[u8],
    bll_bytes: &[u8],
) -> Result<PathBuf> {
    let store = Store::open()?;
    store.put_package(package_name, version, sbl_bytes, bll_bytes)
}

pub fn write_lock(path: &Path, lock: &LockFile) -> Result<()> {
    fs::write(path, lock.to_toon()).with_context(|| format!("writing {}", path.display()))
}

pub fn read_lock(path: &Path) -> Result<LockFile> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    LockFile::from_toon(&text)
}

pub fn ensure_restored(lock: &LockFile) -> Result<()> {
    let store = Store::open()?;
    let reg = FileRegistry::open(registry_root(None))?;
    for pkg in &lock.packages {
        if store.has_package(&pkg.name, &pkg.version, &pkg.sbl) {
            continue;
        }
        // Try file registry when source is registry or path is missing.
        if pkg.source == "registry" || pkg.sbl.is_empty() {
            match reg.fetch_to_store(&pkg.name, &pkg.version) {
                Ok(_) => continue,
                Err(e) => bail!(
                    "package `{}@{}` missing from store and registry fetch failed: {e:#}",
                    pkg.name,
                    pkg.version
                ),
            }
        }
        bail!(
            "package `{}@{}` missing from store (hash {}); run `lang pack` or `lang publish`",
            pkg.name,
            pkg.version,
            pkg.sbl
        );
    }
    Ok(())
}
