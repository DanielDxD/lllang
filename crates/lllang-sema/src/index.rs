//! Position → symbol index for LSP go-to-def / rename / inlays.

use crate::types::Ty;
use lllang_syntax::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Type,
    Local,
    Param,
    Field,
}

#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub id: SymbolId,
    pub name: String,
    pub file: String,
    pub def_span: Span,
    pub ty: Option<String>,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone)]
pub struct SymbolRef {
    pub def: SymbolId,
    pub file: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InlayHint {
    pub file: String,
    /// Insert hint after this span (typically the binding name).
    pub after: Span,
    pub label: String,
}

#[derive(Debug, Default, Clone)]
pub struct SymbolIndex {
    pub defs: Vec<SymbolDef>,
    pub refs: Vec<SymbolRef>,
    pub inlays: Vec<InlayHint>,
    /// (file, name) → most recent def in that file (locals shadow).
    by_name: HashMap<(String, String), SymbolId>,
}

impl SymbolIndex {
    pub fn define(
        &mut self,
        name: impl Into<String>,
        file: impl Into<String>,
        def_span: Span,
        ty: Option<&Ty>,
        kind: SymbolKind,
    ) -> SymbolId {
        let id = SymbolId(self.defs.len());
        let name = name.into();
        let file = file.into();
        self.by_name.insert((file.clone(), name.clone()), id);
        self.defs.push(SymbolDef {
            id,
            name,
            file,
            def_span,
            ty: ty.map(|t| t.display()),
            kind,
        });
        id
    }

    pub fn define_global(
        &mut self,
        name: impl Into<String>,
        file: impl Into<String>,
        def_span: Span,
        ty: Option<&Ty>,
        kind: SymbolKind,
    ) -> SymbolId {
        let id = self.define(name, file, def_span, ty, kind);
        // Also index under empty file key for cross-file lookup by name.
        let name = self.defs[id.0].name.clone();
        self.by_name.insert((String::new(), name), id);
        id
    }

    pub fn reference(&mut self, def: SymbolId, file: impl Into<String>, span: Span) {
        self.refs.push(SymbolRef {
            def,
            file: file.into(),
            span,
        });
    }

    pub fn lookup_name(&self, file: &str, name: &str) -> Option<SymbolId> {
        self.by_name
            .get(&(file.to_string(), name.to_string()))
            .or_else(|| self.by_name.get(&(String::new(), name.to_string())))
            .copied()
    }

    pub fn inlay(&mut self, file: impl Into<String>, after: Span, label: impl Into<String>) {
        self.inlays.push(InlayHint {
            file: file.into(),
            after,
            label: label.into(),
        });
    }

    /// Find the symbol whose def or ref span covers `offset` in `file`.
    pub fn resolve_at(&self, file: &str, offset: u32) -> Option<&SymbolDef> {
        for r in self.refs.iter().rev() {
            if r.file == file && span_contains(r.span, offset) {
                return self.defs.get(r.def.0);
            }
        }
        for d in self.defs.iter().rev() {
            if d.file == file && span_contains(d.def_span, offset) {
                return Some(d);
            }
        }
        None
    }

    pub fn find_references(&self, def: SymbolId) -> Vec<&SymbolRef> {
        self.refs.iter().filter(|r| r.def == def).collect()
    }

    pub fn all_spans_for(&self, def: SymbolId) -> Vec<(String, Span)> {
        let mut out = Vec::new();
        if let Some(d) = self.defs.get(def.0) {
            out.push((d.file.clone(), d.def_span));
        }
        for r in &self.refs {
            if r.def == def {
                out.push((r.file.clone(), r.span));
            }
        }
        out
    }

    pub fn rename_edits(&self, def: SymbolId, new_name: &str) -> Vec<RenameEdit> {
        self.all_spans_for(def)
            .into_iter()
            .map(|(file, span)| RenameEdit {
                file,
                span,
                new_text: new_name.to_string(),
            })
            .collect()
    }

    pub fn inlay_hints_for(&self, file: &str) -> Vec<&InlayHint> {
        self.inlays.iter().filter(|h| h.file == file).collect()
    }

    /// Names visible for bare-identifier completion in `file`.
    pub fn completion_names(&self, file: &str) -> Vec<CompletionItem> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for d in &self.defs {
            let in_file = d.file == file;
            let is_global = self.by_name.contains_key(&(String::new(), d.name.clone()));
            if !in_file && !is_global {
                continue;
            }
            if !seen.insert(d.name.clone()) {
                continue;
            }
            out.push(CompletionItem {
                label: d.name.clone(),
                detail: d
                    .ty
                    .clone()
                    .unwrap_or_else(|| format!("{:?}", d.kind).to_lowercase()),
                kind: match d.kind {
                    SymbolKind::Function => CompletionKind::Function,
                    SymbolKind::Type => CompletionKind::Type,
                    SymbolKind::Field => CompletionKind::Field,
                    SymbolKind::Param | SymbolKind::Local => CompletionKind::Variable,
                },
            });
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Type,
    Field,
    Variable,
    Keyword,
    Method,
    Property,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone)]
pub struct RenameEdit {
    pub file: String,
    pub span: Span,
    pub new_text: String,
}

fn span_contains(span: Span, offset: u32) -> bool {
    offset >= span.start && offset < span.end.max(span.start + 1)
}
