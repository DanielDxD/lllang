//! Minimal TOON-like `Package.lock` read/write.

use anyhow::{bail, Result};

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
    pub source: String,
    pub targets: Vec<LockedTarget>,
}

#[derive(Debug, Clone)]
pub struct LockedTarget {
    pub triple: String,
    pub bll: String,
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
            out.push_str(&format!("    source: {}\n", p.source));
            out.push_str(&format!("    targets[{}]:\n", p.targets.len()));
            for t in &p.targets {
                out.push_str(&format!("      - triple: {}\n", t.triple));
                out.push_str(&format!("        bll: {}\n", t.bll));
            }
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
            if let Some(v) = t.strip_prefix("version:") {
                lock.version = v.trim().parse().unwrap_or(1);
                continue;
            }
            if t.starts_with("packages[") {
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
                    source: String::new(),
                    targets: vec![],
                });
                in_targets = false;
                continue;
            }
            let Some(pkg) = cur.as_mut() else {
                continue;
            };
            if t.starts_with("version:") {
                pkg.version = t.trim_start_matches("version:").trim().to_string();
            } else if t.starts_with("sbl:") {
                pkg.sbl = t.trim_start_matches("sbl:").trim().to_string();
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
        if lock.packages.is_empty() && !text.trim().is_empty() && !text.contains("packages[0]") {
            // Allow empty packages list.
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
                source: "local".into(),
                targets: vec![LockedTarget {
                    triple: "aarch64-apple-darwin".into(),
                    bll: "def".into(),
                }],
            }],
        };
        let text = lock.to_toon();
        let parsed = LockFile::from_toon(&text).unwrap();
        assert_eq!(parsed.packages[0].name, "std.json");
        assert_eq!(parsed.packages[0].targets[0].bll, "def");
    }
}
