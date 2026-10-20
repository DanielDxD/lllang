//! Document analysis helpers for LSP features.

use lllang_syntax::{
    Access, ClassDecl, Decl, EnumDecl, File, FuncDecl, MemberDecl, ParseResult, ProtocolDecl,
    SourceMap, Span, StructDecl, Severity, Diagnostic, Pattern, LetDecl, VarDecl,
};
use tower_lsp::lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, DocumentSymbol, FoldingRange,
    FoldingRangeKind, Position, Range, SelectionRange, SymbolKind, Url,
};

#[derive(Debug)]
pub struct AnalyzedDocument {
    #[allow(dead_code)]
    pub uri: Url,
    pub source: String,
    pub map: SourceMap,
    pub parsed: ParseResult,
}

impl AnalyzedDocument {
    pub fn new(uri: Url, source: String) -> Self {
        let map = SourceMap::new(&source);
        let parsed = lllang_syntax::parse(&source);
        Self {
            uri,
            source,
            map,
            parsed,
        }
    }

    pub fn lsp_diagnostics(&self) -> Vec<LspDiagnostic> {
        let mut out: Vec<LspDiagnostic> = self
            .parsed
            .diagnostics
            .iter()
            .map(|d| diagnostic_to_lsp(d, &self.map))
            .collect();
        // Phase 2: also run sema (skip manifests).
        let path = self.uri.path().to_string();
        let name = path.rsplit('/').next().unwrap_or("");
        if name != "Package.ll" && name != "Solution.ll" {
            let sema = lllang_sema::analyze(&[(path, self.source.clone())]);
            for d in &sema.diagnostics {
                out.push(diagnostic_to_lsp(d, &self.map));
            }
        }
        out
    }

    pub fn document_symbols(&self) -> Vec<DocumentSymbol> {
        self.parsed
            .file
            .decls
            .iter()
            .flat_map(|d| decl_symbols(d, &self.map))
            .collect()
    }

    pub fn folding_ranges(&self) -> Vec<FoldingRange> {
        let mut ranges = Vec::new();
        collect_folds(&self.parsed.file, &self.map, &mut ranges);
        ranges
    }

    pub fn selection_ranges(&self, positions: &[Position]) -> Vec<SelectionRange> {
        positions
            .iter()
            .map(|p| {
                let offset = position_to_offset(&self.source, *p);
                let span = enclosing_span(&self.parsed.file, offset).unwrap_or(Span::EMPTY);
                SelectionRange {
                    range: span_to_range(span, &self.map),
                    parent: None,
                }
            })
            .collect()
    }

    pub fn hover_text(&self, position: Position) -> Option<String> {
        let offset = position_to_offset(&self.source, position);
        let word = word_at(&self.source, offset)?;
        if is_keyword(&word) {
            return Some(format!("**keyword** `{word}`"));
        }
        if let Some(desc) = find_symbol_desc(&self.parsed.file, &word) {
            return Some(desc);
        }
        Some(format!("`{word}`"))
    }

    pub fn completions(&self) -> Vec<(String, String)> {
        let mut items: Vec<(String, String)> = KEYWORDS
            .iter()
            .map(|k| ((*k).to_string(), "keyword".to_string()))
            .collect();
        collect_names(&self.parsed.file, &mut items);
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items.dedup_by(|a, b| a.0 == b.0);
        items
    }
}

fn diagnostic_to_lsp(d: &Diagnostic, map: &SourceMap) -> LspDiagnostic {
    let severity = match d.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::INFORMATION,
    };
    LspDiagnostic {
        range: span_to_range(d.span, map),
        severity: Some(severity),
        code: Some(tower_lsp::lsp_types::NumberOrString::String(d.code.clone())),
        source: Some("lllang".into()),
        message: d.message.clone(),
        ..Default::default()
    }
}

pub fn span_to_range(span: Span, map: &SourceMap) -> Range {
    let start = map.loc(span.start);
    let end = map.loc(span.end);
    Range {
        start: Position {
            line: start.line.saturating_sub(1),
            character: start.column.saturating_sub(1),
        },
        end: Position {
            line: end.line.saturating_sub(1),
            character: end.column.saturating_sub(1),
        },
    }
}

pub fn position_to_offset(source: &str, pos: Position) -> u32 {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if line == pos.line && col == pos.character {
            return i as u32;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    source.len() as u32
}

fn word_at(source: &str, offset: u32) -> Option<String> {
    let bytes = source.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut i = (offset as usize).min(bytes.len().saturating_sub(1));
    while i > 0 && is_ident_byte(bytes[i]) {
        i -= 1;
    }
    if !is_ident_byte(bytes[i]) {
        i += 1;
    }
    let start = i;
    while i < bytes.len() && is_ident_byte(bytes[i]) {
        i += 1;
    }
    if start >= i {
        None
    } else {
        Some(source[start..i].to_string())
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

const KEYWORDS: &[&str] = &[
    "namespace", "import", "public", "private", "protected", "internal", "struct", "class",
    "enum", "protocol", "extension", "func", "let", "var", "if", "else", "guard", "while",
    "for", "in", "return", "throw", "throws", "try", "catch", "do", "defer", "switch", "where",
    "is", "as", "async", "await", "weak", "unowned", "override", "virtual", "abstract", "sealed",
    "init", "deinit", "self", "Self", "inout", "get", "set", "didSet", "unsafe", "true", "false",
    "nil",
];

fn is_keyword(w: &str) -> bool {
    KEYWORDS.contains(&w)
}

fn access_label(a: Option<Access>) -> &'static str {
    match a {
        Some(Access::Public) => "public ",
        Some(Access::Private) => "private ",
        Some(Access::Protected) => "protected ",
        Some(Access::Internal) => "internal ",
        Some(Access::Fileprivate) => "fileprivate ",
        None => "",
    }
}

fn pattern_names(p: &Pattern) -> Vec<String> {
    match p {
        Pattern::Ident { name, .. } => vec![name.clone()],
        Pattern::Wildcard { .. } | Pattern::Literal { .. } | Pattern::Error { .. } => vec![],
        Pattern::Typed { pattern, .. } | Pattern::Binding { pattern, .. } => pattern_names(pattern),
        Pattern::Tuple { elements, .. } => elements.iter().flat_map(pattern_names).collect(),
        Pattern::EnumCase { subpatterns, .. } => subpatterns.iter().flat_map(pattern_names).collect(),
    }
}

fn binding_names(bindings: &[lllang_syntax::PatternBinding]) -> Vec<(String, Span)> {
    bindings
        .iter()
        .flat_map(|b| {
            pattern_names(&b.pattern)
                .into_iter()
                .map(|n| (n, b.span))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn decl_symbols(decl: &Decl, map: &SourceMap) -> Vec<DocumentSymbol> {
    match decl {
        Decl::Struct(s) => vec![type_symbol(&s.name, s.span, SymbolKind::STRUCT, &s.members, map)],
        Decl::Class(c) => vec![type_symbol(&c.name, c.span, SymbolKind::CLASS, &c.members, map)],
        Decl::Enum(e) => {
            let mut children: Vec<DocumentSymbol> = e
                .cases
                .iter()
                .map(|c| bare_symbol(&c.name, c.span, SymbolKind::ENUM_MEMBER, map))
                .collect();
            children.extend(e.members.iter().filter_map(|m| member_symbol(m, map)));
            vec![DocumentSymbol {
                name: e.name.clone(),
                detail: Some(format!("{}enum", access_label(e.access))),
                kind: SymbolKind::ENUM,
                tags: None,
                #[allow(deprecated)]
                deprecated: None,
                range: span_to_range(e.span, map),
                selection_range: span_to_range(e.span, map),
                children: Some(children),
            }]
        }
        Decl::Protocol(p) => {
            vec![type_symbol(&p.name, p.span, SymbolKind::INTERFACE, &p.members, map)]
        }
        Decl::Func(f) => vec![func_symbol(f, map)],
        Decl::Let(l) => let_symbols(l, map),
        Decl::Var(v) => var_symbols(v, map),
        Decl::Const(c) => vec![bare_symbol(&c.name, c.span, SymbolKind::CONSTANT, map)],
        Decl::Namespace(n) => vec![bare_symbol(
            &n.path.segments.join("."),
            n.span,
            SymbolKind::NAMESPACE,
            map,
        )],
        Decl::Import(_) | Decl::Extension(_) | Decl::Attribute(_) => vec![],
    }
}

fn let_symbols(l: &LetDecl, map: &SourceMap) -> Vec<DocumentSymbol> {
    binding_names(&l.bindings)
        .into_iter()
        .map(|(n, span)| bare_symbol(&n, span, SymbolKind::VARIABLE, map))
        .collect()
}

fn var_symbols(v: &VarDecl, map: &SourceMap) -> Vec<DocumentSymbol> {
    binding_names(&v.bindings)
        .into_iter()
        .map(|(n, span)| bare_symbol(&n, span, SymbolKind::VARIABLE, map))
        .collect()
}

fn type_symbol(
    name: &str,
    span: Span,
    kind: SymbolKind,
    members: &[MemberDecl],
    map: &SourceMap,
) -> DocumentSymbol {
    let children = members
        .iter()
        .filter_map(|m| member_symbol(m, map))
        .collect();
    DocumentSymbol {
        name: name.to_string(),
        detail: None,
        kind,
        tags: None,
        #[allow(deprecated)]
        deprecated: None,
        range: span_to_range(span, map),
        selection_range: span_to_range(span, map),
        children: Some(children),
    }
}

fn member_symbol(m: &MemberDecl, map: &SourceMap) -> Option<DocumentSymbol> {
    match m {
        MemberDecl::Func(f) => Some(func_symbol(f, map)),
        MemberDecl::Let(l) => let_symbols(l, map).into_iter().next(),
        MemberDecl::Var(v) => var_symbols(v, map).into_iter().next(),
        MemberDecl::Init(i) => Some(bare_symbol("init", i.span, SymbolKind::CONSTRUCTOR, map)),
        MemberDecl::Deinit(d) => Some(bare_symbol("deinit", d.span, SymbolKind::METHOD, map)),
        MemberDecl::Nested(inner) => decl_symbols(inner, map).into_iter().next(),
    }
}

fn func_symbol(f: &FuncDecl, map: &SourceMap) -> DocumentSymbol {
    DocumentSymbol {
        name: f.name.clone(),
        detail: Some(format!("{}func", access_label(f.access))),
        kind: SymbolKind::FUNCTION,
        tags: None,
        #[allow(deprecated)]
        deprecated: None,
        range: span_to_range(f.span, map),
        selection_range: span_to_range(f.span, map),
        children: None,
    }
}

fn bare_symbol(name: &str, span: Span, kind: SymbolKind, map: &SourceMap) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: None,
        kind,
        tags: None,
        #[allow(deprecated)]
        deprecated: None,
        range: span_to_range(span, map),
        selection_range: span_to_range(span, map),
        children: None,
    }
}

fn collect_folds(file: &File, map: &SourceMap, out: &mut Vec<FoldingRange>) {
    for d in &file.decls {
        let span = match d {
            Decl::Struct(StructDecl { span, .. })
            | Decl::Class(ClassDecl { span, .. })
            | Decl::Enum(EnumDecl { span, .. })
            | Decl::Protocol(ProtocolDecl { span, .. })
            | Decl::Func(FuncDecl { span, .. }) => Some(*span),
            _ => None,
        };
        if let Some(span) = span {
            let range = span_to_range(span, map);
            if range.start.line < range.end.line {
                out.push(FoldingRange {
                    start_line: range.start.line,
                    start_character: Some(range.start.character),
                    end_line: range.end.line,
                    end_character: Some(range.end.character),
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: None,
                });
            }
        }
    }
}

fn enclosing_span(file: &File, offset: u32) -> Option<Span> {
    let mut best: Option<Span> = None;
    for d in &file.decls {
        let span = decl_span(d);
        if span.start <= offset && offset <= span.end {
            best = Some(match best {
                Some(b) if (span.end - span.start) < (b.end - b.start) => span,
                Some(b) => b,
                None => span,
            });
        }
    }
    best
}

fn decl_span(d: &Decl) -> Span {
    match d {
        Decl::Namespace(n) => n.span,
        Decl::Import(i) => i.span,
        Decl::Struct(s) => s.span,
        Decl::Class(c) => c.span,
        Decl::Enum(e) => e.span,
        Decl::Protocol(p) => p.span,
        Decl::Extension(e) => e.span,
        Decl::Func(f) => f.span,
        Decl::Let(l) => l.span,
        Decl::Var(v) => v.span,
        Decl::Const(c) => c.span,
        Decl::Attribute(a) => a.span,
    }
}

fn find_symbol_desc(file: &File, name: &str) -> Option<String> {
    for d in &file.decls {
        match d {
            Decl::Struct(s) if s.name == name => return Some(format!("```ll\nstruct {name}\n```")),
            Decl::Class(c) if c.name == name => return Some(format!("```ll\nclass {name}\n```")),
            Decl::Enum(e) if e.name == name => return Some(format!("```ll\nenum {name}\n```")),
            Decl::Protocol(p) if p.name == name => {
                return Some(format!("```ll\nprotocol {name}\n```"))
            }
            Decl::Func(f) if f.name == name => return Some(format!("```ll\nfunc {name}(...)\n```")),
            _ => {}
        }
    }
    None
}

fn collect_names(file: &File, out: &mut Vec<(String, String)>) {
    for d in &file.decls {
        match d {
            Decl::Struct(s) => out.push((s.name.clone(), "struct".into())),
            Decl::Class(c) => out.push((c.name.clone(), "class".into())),
            Decl::Enum(e) => {
                out.push((e.name.clone(), "enum".into()));
                for c in &e.cases {
                    out.push((c.name.clone(), "enum case".into()));
                }
            }
            Decl::Protocol(p) => out.push((p.name.clone(), "protocol".into())),
            Decl::Func(f) => out.push((f.name.clone(), "function".into())),
            Decl::Let(l) => {
                for (n, _) in binding_names(&l.bindings) {
                    out.push((n, "let".into()));
                }
            }
            Decl::Var(v) => {
                for (n, _) in binding_names(&v.bindings) {
                    out.push((n, "var".into()));
                }
            }
            _ => {}
        }
    }
}
