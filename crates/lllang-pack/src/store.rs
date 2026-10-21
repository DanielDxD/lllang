//! Global content-addressed store under `LANG_HOME` / `~/.lang`.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub fn lang_home() -> PathBuf {
    if let Ok(h) = std::env::var("LANG_HOME") {
        return PathBuf::from(h);
    }
    dirs_home().join(".lang")
}

fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn store_path() -> PathBuf {
    lang_home().join("store")
}

pub fn host_triple() -> String {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    match (arch, os) {
        ("aarch64", "macos") => "aarch64-apple-darwin".into(),
        ("x86_64", "macos") => "x86_64-apple-darwin".into(),
        ("x86_64", "linux") => "x86_64-unknown-linux-gnu".into(),
        ("aarch64", "linux") => "aarch64-unknown-linux-gnu".into(),
        (a, o) => format!("{a}-unknown-{o}"),
    }
}

pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn open() -> Result<Self> {
        let root = store_path();
        fs::create_dir_all(root.join("blobs")).ok();
        fs::create_dir_all(root.join("packages")).ok();
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn put_blob(&self, hash: &str, bytes: &[u8]) -> Result<PathBuf> {
        let prefix = &hash[..hash.len().min(2)];
        let dir = self.root.join("blobs").join(prefix);
        fs::create_dir_all(&dir)?;
        let path = dir.join(hash);
        if !path.exists() {
            fs::write(&path, bytes)?;
        }
        Ok(path)
    }

    pub fn put_package(
        &self,
        name: &str,
        version: &str,
        sbl: &[u8],
        bll: &[u8],
    ) -> Result<PathBuf> {
        let sbl_hash = crate::content_hash(sbl);
        let bll_hash = crate::content_hash(bll);
        self.put_blob(&sbl_hash, sbl)?;
        self.put_blob(&bll_hash, bll)?;
        let entry = self
            .root
            .join("packages")
            .join(name)
            .join(version);
        fs::create_dir_all(entry.join("targets"))?;
        fs::write(entry.join("package.sbl"), sbl)?;
        fs::write(
            entry.join("targets").join(format!("{}.bll", host_triple())),
            bll,
        )?;
        fs::write(
            entry.join("META"),
            format!("sbl={sbl_hash}\nbll={bll_hash}\ntriple={}\n", host_triple()),
        )?;
        Ok(entry)
    }

    pub fn has_package(&self, name: &str, version: &str, sbl_hash: &str) -> bool {
        let meta = self
            .root
            .join("packages")
            .join(name)
            .join(version)
            .join("META");
        let Ok(text) = fs::read_to_string(meta) else {
            return false;
        };
        text.contains(&format!("sbl={sbl_hash}"))
            || sbl_hash.is_empty()
            || text.lines().any(|l| l.starts_with("sbl="))
    }

    pub fn package_sbl_path(&self, name: &str, version: &str) -> PathBuf {
        self.root
            .join("packages")
            .join(name)
            .join(version)
            .join("package.sbl")
    }

    pub fn package_bll_path(&self, name: &str, version: &str) -> PathBuf {
        self.root
            .join("packages")
            .join(name)
            .join(version)
            .join("targets")
            .join(format!("{}.bll", host_triple()))
    }

    pub fn load_sbl(&self, name: &str, version: &str) -> Result<String> {
        let p = self.package_sbl_path(name, version);
        fs::read_to_string(&p).with_context(|| format!("loading {}", p.display()))
    }
}
