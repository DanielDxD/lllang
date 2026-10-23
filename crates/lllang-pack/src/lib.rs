//! Pack `.sbl` / `.bll`, global store, and `Package.lock` (Phase 3/9 MVP).

use anyhow::{bail, Context, Result};
use lllang_syntax::{parse, Access, Decl, FuncDecl, MemberDecl};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub mod bll;
pub mod lock;
pub mod registry;
pub mod store;
pub mod target;

pub use bll::{decode_portable_bll, encode_portable_bll, is_portable_bll, PortableBll};
pub use lock::{LockFile, LockedPackage, LockedTarget};
pub use registry::{registry_root, FileRegistry, RegistryPackage};
pub use store::{host_triple, lang_home, store_path, Store};
pub use target::{
    is_official_triple, object_ext, resolve_build_target, resolve_target_spec, runtime_archive_name,
    OFFICIAL_TRIPLES, PORTABLE_TRIPLE,
};

/// Emit a textual `.sbl` surface from public declarations in sources.
/// Output is a subset of `.ll` that the parser accepts (bodies omitted).
pub fn emit_sbl(package_name: &str, version: &str, sources: &[(String, String)]) -> Result<String> {
    let mut out = String::new();
    out.push_str("// sbl-format: 1\n");
    out.push_str(&format!("// package {package_name}@{version}\n\n"));
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

/// Strip pack metadata comments / `package` header so the text is analyzable as `.ll`.
pub fn sbl_to_analysis_source(sbl: &str) -> String {
    let mut out = String::new();
    for line in sbl.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        if t.starts_with("package ") || t.starts_with("abi ") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn is_public(access: Option<Access>) -> bool {
    matches!(access, Some(Access::Public | Access::Open) | None)
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
                let items: Vec<_> = inh.items.iter().map(type_approx).collect();
                line.push_str(": ");
                line.push_str(&items.join(", "));
            }
            let mut body = String::new();
            for m in &s.members {
                if let MemberDecl::Func(f) = m {
                    if is_public(f.access) {
                        body.push_str("    ");
                        body.push_str(&func_sig_sbl(f));
                        body.push('\n');
                    }
                }
            }
            if body.is_empty() {
                Some(format!("{line} {{}}"))
            } else {
                Some(format!("{line} {{\n{body}}}"))
            }
        }
        Decl::Class(c) if is_public(c.access) => {
            let mut line = format!("public class {}", c.name);
            if !c.primary_ctor.is_empty() {
                let params: Vec<_> = c
                    .primary_ctor
                    .iter()
                    .map(|p| format!("{}: {}", p.name, type_approx(&p.ty)))
                    .collect();
                line.push('(');
                line.push_str(&params.join(", "));
                line.push(')');
            }
            let mut body = String::new();
            for m in &c.members {
                if let MemberDecl::Func(f) = m {
                    if is_public(f.access) {
                        body.push_str("    ");
                        body.push_str(&func_sig_sbl(f));
                        body.push('\n');
                    }
                }
            }
            if body.is_empty() {
                Some(format!("{line} {{}}"))
            } else {
                Some(format!("{line} {{\n{body}}}"))
            }
        }
        Decl::Enum(e) if is_public(e.access) => {
            let cases: Vec<_> = e
                .cases
                .iter()
                .map(|c| {
                    if c.payload.is_empty() {
                        c.name.clone()
                    } else {
                        let fs: Vec<_> = c
                            .payload
                            .iter()
                            .map(|(n, t)| format!("{n}: {}", type_approx(t)))
                            .collect();
                        format!("{}({})", c.name, fs.join(", "))
                    }
                })
                .collect();
            Some(format!(
                "public enum {} {{\n    {}\n}}",
                e.name,
                cases.join(",\n    ")
            ))
        }
        Decl::Protocol(p) if is_public(p.access) => {
            let mut body = String::new();
            for m in &p.members {
                if let MemberDecl::Func(f) = m {
                    body.push_str("    ");
                    body.push_str(&func_sig_sbl(f));
                    body.push('\n');
                }
            }
            Some(format!("public protocol {} {{\n{body}}}", p.name))
        }
        Decl::Func(f) if is_public(f.access) => Some(func_sig_sbl(f)),
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

/// Pack a library: write `package.sbl` and portable `package.bll` (BLL v2 bitcode) under `out_dir`.
///
/// `bitcode` is raw LLVM bitcode; it is wrapped in a BLL v2 container before writing.
pub fn pack_library(
    package_name: &str,
    version: &str,
    sources: &[(String, String)],
    bitcode: &[u8],
    out_dir: &Path,
) -> Result<(PathBuf, PathBuf, String, String)> {
    let sbl = emit_sbl(package_name, version, sources)?;
    let sbl_hash = content_hash(sbl.as_bytes());
    let bll_bytes = encode_portable_bll(package_name, version, bitcode);
    let bll_hash = content_hash(&bll_bytes);
    fs::create_dir_all(out_dir)?;
    let sbl_path = out_dir.join("package.sbl");
    let bll_path = out_dir.join("package.bll");
    fs::write(&bll_path, &bll_bytes)?;
    let mut sbl_with_footer = sbl;
    sbl_with_footer.push_str(&format!(
        "\n// integrity: sbl={sbl_hash} bll={bll_hash} triple={PORTABLE_TRIPLE}\n"
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
        let in_store = store.has_package(&pkg.name, &pkg.version, &pkg.sbl);
        if !in_store {
            if pkg.source == "registry" || pkg.sbl.is_empty() {
                match reg.fetch_to_store(&pkg.name, &pkg.version) {
                    Ok(_) => {}
                    Err(e) => bail!(
                        "package `{}@{}` missing from store and registry fetch failed: {e:#}",
                        pkg.name,
                        pkg.version
                    ),
                }
            } else {
                bail!(
                    "package `{}@{}` missing from store (hash {}); run `lang pack` or `lang publish`",
                    pkg.name,
                    pkg.version,
                    pkg.sbl
                );
            }
        }
        verify_lock_hashes(&store, pkg)?;
    }
    Ok(())
}

/// Hard-fail when stored `.sbl` / portable `.bll` bytes do not match lock hashes.
pub fn verify_lock_hashes(store: &Store, pkg: &LockedPackage) -> Result<()> {
    if !pkg.sbl.is_empty() {
        let path = store.package_sbl_path(&pkg.name, &pkg.version);
        if path.exists() {
            let h = content_hash(&fs::read(&path)?);
            if h != pkg.sbl {
                bail!(
                    "hash mismatch for `{}@{}` sbl: store has {h}, lock expects {}",
                    pkg.name,
                    pkg.version,
                    pkg.sbl
                );
            }
        }
    }
    let expected_bll = pkg.portable_bll_hash();
    if let Some(expected) = expected_bll {
        if !expected.is_empty() {
            let path = store.package_bll_path(&pkg.name, &pkg.version);
            if path.exists() {
                let bytes = fs::read(&path)?;
                if !is_portable_bll(&bytes) {
                    bail!(
                        "package `{}@{}` has non-portable `.bll`; rebuild with `lang pack` (BLL v2)",
                        pkg.name,
                        pkg.version
                    );
                }
                let h = content_hash(&bytes);
                if h != expected {
                    bail!(
                        "hash mismatch for `{}@{}` bll: store has {h}, lock expects {expected}",
                        pkg.name,
                        pkg.version
                    );
                }
            }
        }
    }
    Ok(())
}

/// Resolve `name@version` from store or file registry into a locked package entry.
pub fn resolve_locked_package(name: &str, version: &str) -> Result<LockedPackage> {
    let store = Store::open()?;
    let sbl_path = store.package_sbl_path(name, version);
    if !sbl_path.exists() {
        let reg = FileRegistry::open(registry_root(None))?;
        reg.fetch_to_store(name, version)?;
    }
    let sbl_bytes = fs::read(store.package_sbl_path(name, version))?;
    let bll_path = store.package_bll_path(name, version);
    if !bll_path.exists() {
        bail!("package `{name}@{version}` has no portable .bll in store");
    }
    let bll_bytes = fs::read(&bll_path)?;
    if !is_portable_bll(&bll_bytes) {
        bail!("package `{name}@{version}` .bll is not BLL v2 portable; rebuild with `lang pack`");
    }
    Ok(LockedPackage {
        name: name.into(),
        version: version.into(),
        sbl: content_hash(&sbl_bytes),
        source: "registry".into(),
        bll: content_hash(&bll_bytes),
        targets: vec![LockedTarget {
            triple: PORTABLE_TRIPLE.into(),
            bll: content_hash(&bll_bytes),
        }],
    })
}

/// True when `~/.lang/sdk/<triple>/VERSION` (or equivalent LANG_HOME) exists.
pub fn sdk_triple_installed(triple: &str) -> bool {
    lang_home()
        .join("sdk")
        .join(triple)
        .join("VERSION")
        .is_file()
}

/// Resolve store `.sbl` analysis units and portable `.bll` paths for locked / registry deps.
///
/// Returned `.bll` paths are BLL v2 containers (not native objects). Callers must materialize
/// them for the active build triple before linking.
pub fn resolve_prebuilt_deps(
    lock: Option<&LockFile>,
    registry_deps: &[(String, String)],
) -> Result<(Vec<(String, String)>, Vec<PathBuf>)> {
    let store = Store::open()?;
    let mut sbl_units = Vec::new();
    let mut bll_paths = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut specs: Vec<(String, String)> = registry_deps.to_vec();
    if let Some(lock) = lock {
        for p in &lock.packages {
            if !p.source.starts_with("path:") {
                specs.push((p.name.clone(), p.version.clone()));
            }
        }
    }

    for (name, version) in specs {
        let key = format!("{name}@{version}");
        if !seen.insert(key.clone()) {
            continue;
        }
        let sbl_path = store.package_sbl_path(&name, &version);
        if !sbl_path.exists() {
            bail!("prebuilt package `{name}@{version}` not in store (missing package.sbl)");
        }
        let sbl = fs::read_to_string(&sbl_path)?;
        sbl_units.push((
            format!("store:{name}@{version}/package.sbl"),
            sbl_to_analysis_source(&sbl),
        ));
        let bll = store.package_bll_path(&name, &version);
        if bll.exists() {
            let bytes = fs::read(&bll)?;
            if !is_portable_bll(&bytes) {
                bail!(
                    "prebuilt package `{name}@{version}` has non-portable `.bll`; \
                     rebuild with `lang pack` (BLL v2 portable)"
                );
            }
            bll_paths.push(bll);
        } else {
            bail!("prebuilt package `{name}@{version}` missing portable .bll");
        }
    }
    Ok((sbl_units, bll_paths))
}
