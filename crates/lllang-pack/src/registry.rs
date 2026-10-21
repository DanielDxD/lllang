//! File-backed registry client (Phase 4 MVP).
//!
//! Layout under a registry root directory (or `file://` path):
//! ```text
//! <root>/
//!   packages/<name>/<version>/
//!     meta.toon          # name, version, sbl_hash, bll_hash
//!   blobs/<sha256>       # raw .sbl / .bll bytes
//!   index.toon           # flat list of name@version for search
//! ```

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve registry root from CLI/`LANG_REGISTRY`/default `~/.lang/registry`.
pub fn registry_root(explicit: Option<&str>) -> PathBuf {
    if let Some(r) = explicit {
        return normalize_registry_path(r);
    }
    if let Ok(r) = std::env::var("LANG_REGISTRY") {
        if !r.is_empty() {
            return normalize_registry_path(&r);
        }
    }
    crate::lang_home().join("registry")
}

fn normalize_registry_path(s: &str) -> PathBuf {
    let s = s.strip_prefix("file://").unwrap_or(s);
    PathBuf::from(s)
}

#[derive(Debug, Clone)]
pub struct RegistryPackage {
    pub name: String,
    pub version: String,
    pub sbl_hash: String,
    pub bll_hash: String,
}

pub struct FileRegistry {
    pub root: PathBuf,
}

impl FileRegistry {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(root.join("packages"))?;
        fs::create_dir_all(root.join("blobs"))?;
        Ok(Self { root })
    }

    pub fn publish(
        &self,
        name: &str,
        version: &str,
        sbl_bytes: &[u8],
        bll_bytes: &[u8],
    ) -> Result<RegistryPackage> {
        let sbl_hash = hex_sha256(sbl_bytes);
        let bll_hash = hex_sha256(bll_bytes);
        self.put_blob(&sbl_hash, sbl_bytes)?;
        self.put_blob(&bll_hash, bll_bytes)?;
        let dir = self.root.join("packages").join(name).join(version);
        fs::create_dir_all(&dir)?;
        let meta = format!(
            "name: {name}\nversion: {version}\nsbl: {sbl_hash}\nbll: {bll_hash}\n"
        );
        fs::write(dir.join("meta.toon"), meta)?;
        self.rebuild_index()?;
        Ok(RegistryPackage {
            name: name.into(),
            version: version.into(),
            sbl_hash,
            bll_hash,
        })
    }

    pub fn search(&self, query: &str) -> Result<Vec<RegistryPackage>> {
        let mut out = Vec::new();
        let pkgs = self.root.join("packages");
        if !pkgs.exists() {
            return Ok(out);
        }
        for name_ent in fs::read_dir(&pkgs)? {
            let name_ent = name_ent?;
            let name = name_ent.file_name().to_string_lossy().to_string();
            if !query.is_empty() && !name.contains(query) {
                continue;
            }
            if !name_ent.path().is_dir() {
                continue;
            }
            for ver_ent in fs::read_dir(name_ent.path())? {
                let ver_ent = ver_ent?;
                let version = ver_ent.file_name().to_string_lossy().to_string();
                let meta_path = ver_ent.path().join("meta.toon");
                if let Ok(pkg) = read_meta(&meta_path, &name, &version) {
                    out.push(pkg);
                }
            }
        }
        out.sort_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
        Ok(out)
    }

    pub fn fetch_to_store(&self, name: &str, version: &str) -> Result<PathBuf> {
        let meta_path = self
            .root
            .join("packages")
            .join(name)
            .join(version)
            .join("meta.toon");
        let pkg = read_meta(&meta_path, name, version)?;
        let sbl = self.get_blob(&pkg.sbl_hash)?;
        let bll = self.get_blob(&pkg.bll_hash)?;
        crate::install_to_store(name, version, &sbl, &bll)
    }

    fn put_blob(&self, hash: &str, bytes: &[u8]) -> Result<()> {
        let p = self.root.join("blobs").join(hash);
        if !p.exists() {
            fs::write(&p, bytes).with_context(|| format!("writing blob {}", p.display()))?;
        }
        Ok(())
    }

    fn get_blob(&self, hash: &str) -> Result<Vec<u8>> {
        let p = self.root.join("blobs").join(hash);
        fs::read(&p).with_context(|| format!("missing blob {hash}"))
    }

    fn rebuild_index(&self) -> Result<()> {
        let mut lines = Vec::new();
        for pkg in self.search("")? {
            lines.push(format!("{}@{}", pkg.name, pkg.version));
        }
        fs::write(self.root.join("index.toon"), lines.join("\n") + "\n")?;
        Ok(())
    }
}

fn read_meta(path: &Path, name: &str, version: &str) -> Result<RegistryPackage> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut sbl_hash = String::new();
    let mut bll_hash = String::new();
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("sbl: ") {
            sbl_hash = v.trim().into();
        } else if let Some(v) = line.strip_prefix("bll: ") {
            bll_hash = v.trim().into();
        }
    }
    if sbl_hash.is_empty() || bll_hash.is_empty() {
        bail!("incomplete meta in {}", path.display());
    }
    Ok(RegistryPackage {
        name: name.into(),
        version: version.into(),
        sbl_hash,
        bll_hash,
    })
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}
