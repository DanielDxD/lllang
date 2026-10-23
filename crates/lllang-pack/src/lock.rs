//! Minimal TOON-like `Package.lock` read/write.

use anyhow::{bail, Result};
use crate::target::PORTABLE_TRIPLE;

#[derive(Debug, Clone, Default)]
pub struct LockFile {
    pub version: u32,
    pub packages: Vec<LockedPackage>,
}

#[derive(Debug, Clone)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub sbl: String,
    /// Portable `.bll` content hash (preferred). Empty when only legacy `targets[]` is present.
    pub bll: String,
    pub source: String,
    /// Legacy / display: typically one entry with `triple: portable`.
    pub targets: Vec<LockedTarget>,
}

#[derive(Debug, Clone)]
pub struct LockedTarget {
    pub triple: String,
    pub bll: String,
}

impl LockedPackage {
    /// Resolve the portable `.bll` hash from `bll` field or `targets` (portable / any single entry).
    pub fn portable_bll_hash(&self) -> Option<&str> {
        if !self.bll.is_empty() {
            return Some(self.bll.as_str());
        }
        if let Some(t) = self
            .targets
            .iter()
            .find(|t| t.triple == PORTABLE_TRIPLE && !t.bll.is_empty())
        {
            return Some(t.bll.as_str());
        }
        // Migration: if exactly one target hash exists, treat it as the portable hash
        // (consumer still requires BLL v2 magic on disk).
        if self.targets.len() == 1 && !self.targets[0].bll.is_empty() {
            return Some(self.targets[0].bll.as_str());
        }
        self.targets
            .iter()
            .find(|t| !t.bll.is_empty())
            .map(|t| t.bll.as_str())
    }
}

impl LockFile {
    pub fn to_toon(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("version: {}\n", self.version.max(1)));
        out.push_str(&format!("packages[{}]:\n", self.packages.len()));
        for p in &self.packages {
            out.push_str(&format!("  - name: {}\n", p.name));
            out.push_str(&format!("    version: {}\n", p.version));
            out.push_str(&format!("    sbl: {}\n", p.sbl));
            let bll = p
                .portable_bll_hash()
                .unwrap_or("")
                .to_string();
            if !bll.is_empty() {
                out.push_str(&format!("    bll: {bll}\n"));
            }
            out.push_str(&format!("    source: {}\n", p.source));
            // Keep a single portable target row for older readers.
            out.push_str("    targets[1]:\n");
            out.push_str(&format!("      - triple: {PORTABLE_TRIPLE}\n"));
            out.push_str(&format!("        bll: {bll}\n"));
        }
        out
    }

    pub fn from_toon(text: &str) -> Result<Self> {
        let mut lock = LockFile {
            version: 1,
            packages: vec![],
        };
        let mut cur: Option<LockedPackage> = None;
        let mut in_targets = false;
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if t.starts_with("packages[") || t == "packages:" {
                continue;
            }
            if t.starts_with("- name:") {
                if let Some(p) = cur.take() {
                    lock.packages.push(p);
                }
                let name = t.trim_start_matches("- name:").trim().to_string();
                cur = Some(LockedPackage {
                    name,
                    version: String::new(),
                    sbl: String::new(),
                    bll: String::new(),
                    source: String::new(),
                    targets: vec![],
                });
                in_targets = false;
                continue;
            }
            // Schema version only at file scope (before any package entry).
            if cur.is_none() {
                if let Some(v) = t.strip_prefix("version:") {
                    lock.version = v.trim().parse().unwrap_or(1);
                    continue;
                }
            }
            let Some(pkg) = cur.as_mut() else {
                continue;
            };
            if t.starts_with("version:") {
                pkg.version = t.trim_start_matches("version:").trim().to_string();
            } else if t.starts_with("sbl:") {
                pkg.sbl = t.trim_start_matches("sbl:").trim().to_string();
            } else if !in_targets && t.starts_with("bll:") {
                pkg.bll = t.trim_start_matches("bll:").trim().to_string();
            } else if t.starts_with("source:") {
                pkg.source = t.trim_start_matches("source:").trim().to_string();
            } else if t.starts_with("targets[") {
                in_targets = true;
            } else if in_targets && t.starts_with("- triple:") {
                let triple = t.trim_start_matches("- triple:").trim().to_string();
                pkg.targets.push(LockedTarget {
                    triple,
                    bll: String::new(),
                });
            } else if in_targets && t.starts_with("bll:") {
                if let Some(last) = pkg.targets.last_mut() {
                    last.bll = t.trim_start_matches("bll:").trim().to_string();
                }
            }
        }
        if let Some(p) = cur.take() {
            lock.packages.push(p);
        }
        // Backfill package-level bll from targets when missing.
        for p in &mut lock.packages {
            if p.bll.is_empty() {
                if let Some(h) = p
                    .targets
                    .iter()
                    .find(|t| t.triple == PORTABLE_TRIPLE && !t.bll.is_empty())
                    .or_else(|| p.targets.iter().find(|t| !t.bll.is_empty()))
                {
                    p.bll = h.bll.clone();
                }
            }
        }
        if lock.packages.is_empty() && !text.trim().is_empty() && !text.contains("packages[0]") {
            if !text.contains("packages[") {
                bail!("invalid Package.lock: missing packages");
            }
        }
        Ok(lock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let lock = LockFile {
            version: 1,
            packages: vec![LockedPackage {
                name: "std.json".into(),
                version: "0.1.0".into(),
                sbl: "abc".into(),
                bll: "def".into(),
                source: "local".into(),
                targets: vec![LockedTarget {
                    triple: PORTABLE_TRIPLE.into(),
                    bll: "def".into(),
                }],
            }],
        };
        let text = lock.to_toon();
        let parsed = LockFile::from_toon(&text).unwrap();
        assert_eq!(parsed.packages[0].name, "std.json");
        assert_eq!(parsed.packages[0].bll, "def");
        assert_eq!(parsed.packages[0].portable_bll_hash(), Some("def"));
    }
}
