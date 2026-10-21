//! Semantic analysis for LLLang — name resolution and type checking (Phase 2).

mod check;
pub mod hir;
pub mod index;
pub mod prelude;
pub mod types;

pub use index::{
    CompletionItem, CompletionKind, InlayHint, RenameEdit, SymbolDef, SymbolId, SymbolIndex,
    SymbolKind, SymbolRef,
};
pub use lllang_syntax::Diagnostic;
pub use types::Ty;

#[derive(Debug)]
pub struct SemaResult {
    pub module: hir::Module,
    pub diagnostics: Vec<Diagnostic>,
    pub index: SymbolIndex,
}

/// Parse each source file, collect syntax diagnostics, then name-resolve and type-check.
pub fn analyze(sources: &[(String, String)]) -> SemaResult {
    check::analyze_sources(sources)
}

pub fn resolve_at<'a>(result: &'a SemaResult, file: &str, offset: u32) -> Option<&'a SymbolDef> {
    result.index.resolve_at(file, offset)
}

pub fn find_references(result: &SemaResult, def: SymbolId) -> Vec<&SymbolRef> {
    result.index.find_references(def)
}

pub fn rename_edits(result: &SemaResult, def: SymbolId, new_name: &str) -> Vec<RenameEdit> {
    result.index.rename_edits(def, new_name)
}

pub fn inlay_hints_for<'a>(result: &'a SemaResult, file: &str) -> Vec<&'a InlayHint> {
    result.index.inlay_hints_for(file)
}

/// Completions at `offset` in `file`. When `source[offset-1]` is `.`, offer members.
pub fn completions_at(result: &SemaResult, file: &str, source: &str, offset: u32) -> Vec<CompletionItem> {
    let off = offset as usize;
    let before = if off > 0 && off <= source.len() {
        &source[..off]
    } else {
        source
    };
    // Member completion: `recv.` or `recv.pref`
    if let Some(recv) = receiver_before_dot(before) {
        if let Some(ty_name) = resolve_type_name(result, file, &recv) {
            return members_of(result, &ty_name);
        }
        return Vec::new();
    }
    let mut items = result.index.completion_names(file);
    // Keywords as low-priority fallbacks (LSP can still show them).
    for kw in [
        "func", "struct", "class", "enum", "protocol", "import", "let", "var", "if", "else",
        "switch", "case", "return", "throw", "try", "catch", "async", "await", "public", "private",
        "true", "false",
    ] {
        items.push(CompletionItem {
            label: kw.into(),
            detail: "keyword".into(),
            kind: CompletionKind::Keyword,
        });
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    items.dedup_by(|a, b| a.label == b.label);
    items
}

fn receiver_before_dot(before: &str) -> Option<String> {
    let trimmed = before.trim_end();
    let bytes = trimmed.as_bytes();
    let mut i = bytes.len();
    // Skip incomplete member prefix after the last dot.
    while i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        i -= 1;
    }
    if i == 0 || bytes[i - 1] != b'.' {
        return None;
    }
    let dot = i - 1;
    let mut start = dot;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    let recv = trimmed[start..dot].trim();
    if recv.is_empty() {
        None
    } else {
        Some(recv.to_string())
    }
}

fn resolve_type_name(result: &SemaResult, file: &str, name: &str) -> Option<String> {
    if let Some(id) = result.index.lookup_name(file, name) {
        if let Some(def) = result.index.defs.get(id.0) {
            if let Some(ty) = &def.ty {
                return Some(strip_ty_name(ty));
            }
            if def.kind == SymbolKind::Type {
                return Some(def.name.clone());
            }
        }
    }
    None
}

fn strip_ty_name(ty: &str) -> String {
    // `String`, `Product`, `[Int]`, `Foo?` → base name for member lookup
    let t = ty.trim();
    if let Some(inner) = t.strip_suffix('?') {
        return strip_ty_name(inner);
    }
    if let Some(inner) = t.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return format!("Array<{inner}>");
    }
    t.split(|c: char| c == '<' || c == '(' || c == ' ')
        .next()
        .unwrap_or(t)
        .to_string()
}

/// Fields and methods for a named type (HIR + builtin members).
pub fn members_of(result: &SemaResult, type_name: &str) -> Vec<CompletionItem> {
    let mut out = Vec::new();
    for td in &result.module.types {
        let (name, fields, methods) = match td {
            hir::TypeDef::Struct {
                name,
                fields,
                methods,
                ..
            } => (name.as_str(), fields.as_slice(), methods.as_slice()),
            hir::TypeDef::Class {
                name,
                fields,
                methods,
                ..
            } => (name.as_str(), fields.as_slice(), methods.as_slice()),
            hir::TypeDef::Enum { name, cases, .. } => {
                if name == type_name {
                    for c in cases {
                        out.push(CompletionItem {
                            label: c.name.clone(),
                            detail: "case".into(),
                            kind: CompletionKind::Field,
                        });
                    }
                }
                continue;
            }
            hir::TypeDef::Protocol { name, methods, .. } => {
                if name == type_name {
                    for m in methods {
                        out.push(CompletionItem {
                            label: m.name.clone(),
                            detail: m.ret.display(),
                            kind: CompletionKind::Method,
                        });
                    }
                }
                continue;
            }
        };
        if name != type_name {
            continue;
        }
        for (fname, fty) in fields {
            out.push(CompletionItem {
                label: fname.clone(),
                detail: fty.display(),
                kind: CompletionKind::Field,
            });
        }
        for m in methods {
            out.push(CompletionItem {
                label: m.name.clone(),
                detail: m.ret.display(),
                kind: CompletionKind::Method,
            });
        }
    }
    for ((ty, member), (ret, is_prop, _)) in prelude::builtin_members() {
        if ty == type_name {
            out.push(CompletionItem {
                label: member,
                detail: ret.display(),
                kind: if is_prop {
                    CompletionKind::Property
                } else {
                    CompletionKind::Method
                },
            });
        }
    }
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out.dedup_by(|a, b| a.label == b.label);
    out
}
