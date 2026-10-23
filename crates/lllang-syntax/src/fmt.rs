//! Pretty-printer for LLLang (canonical `lang fmt` style).

use crate::ast::*;
use crate::diagnostic::Diagnostic;
use crate::parse;
use crate::span::Span;

const INDENT: &str = "    ";
const MAX_WIDTH: usize = 100;

/// Format source to canonical style. Returns Err with parse diagnostics on failure.
pub fn format_source(source: &str) -> Result<String, Vec<Diagnostic>> {
    let result = parse(source);
    let errors: Vec<_> = result
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == crate::diagnostic::Severity::Error)
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    let comments = collect_comments(source);
    let mut p = Printer::new(comments);
    p.file(&result.file);
    let out = p.finish();
    // Idempotency: second pass should match (comments already in tree via source order).
    Ok(out)
}

#[derive(Debug, Clone)]
struct Comment {
    span: Span,
    text: String,
}

fn collect_comments(source: &str) -> Vec<Comment> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'\'' {
            i += 1;
            if i < bytes.len() && bytes[i] == b'\\' {
                i += 2;
            } else {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'\'' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            out.push(Comment {
                span: Span::new(start as u32, i as u32),
                text: source[start..i].to_string(),
            });
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            let mut depth = 1usize;
            while i + 1 < bytes.len() && depth > 0 {
                if bytes[i] == b'/' && bytes[i + 1] == b'*' {
                    depth += 1;
                    i += 2;
                } else if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            out.push(Comment {
                span: Span::new(start as u32, i as u32),
                text: source[start..i].to_string(),
            });
            continue;
        }
        i += 1;
    }
    out
}

struct Printer {
    out: String,
    indent: usize,
    comments: Vec<Comment>,
    comment_idx: usize,
    at_line_start: bool,
}

impl Printer {
    fn new(comments: Vec<Comment>) -> Self {
        Self {
            out: String::new(),
            indent: 0,
            comments,
            comment_idx: 0,
            at_line_start: true,
        }
    }

    fn finish(mut self) -> String {
        self.emit_remaining_comments();
        if !self.out.ends_with('\n') && !self.out.is_empty() {
            self.out.push('\n');
        }
        self.out
    }

    fn emit_comments_before(&mut self, offset: u32) {
        while self.comment_idx < self.comments.len()
            && self.comments[self.comment_idx].span.start < offset
        {
            let text = self.comments[self.comment_idx].text.clone();
            self.ensure_line_start();
            self.raw(&text);
            self.newline();
            self.comment_idx += 1;
        }
    }

    fn emit_remaining_comments(&mut self) {
        while self.comment_idx < self.comments.len() {
            let text = self.comments[self.comment_idx].text.clone();
            self.ensure_line_start();
            self.raw(&text);
            self.newline();
            self.comment_idx += 1;
        }
    }

    fn ensure_line_start(&mut self) {
        if !self.at_line_start {
            self.newline();
        }
    }

    fn newline(&mut self) {
        self.out.push('\n');
        self.at_line_start = true;
    }

    fn write_indent(&mut self) {
        if self.at_line_start {
            for _ in 0..self.indent {
                self.out.push_str(INDENT);
            }
            self.at_line_start = false;
        }
    }

    fn raw(&mut self, s: &str) {
        self.write_indent();
        self.out.push_str(s);
    }

    fn space(&mut self) {
        if !self.at_line_start && !self.out.ends_with(' ') && !self.out.ends_with('\n') {
            self.out.push(' ');
        }
    }

    fn file(&mut self, file: &File) {
        let mut imports = Vec::new();
        let mut others = Vec::new();
        for d in &file.decls {
            match d {
                Decl::Import(_) => imports.push(d),
                _ => others.push(d),
            }
        }
        imports.sort_by(|a, b| import_key(a).cmp(&import_key(b)));

        let mut first = true;
        // Preserve original order for non-imports, but emit imports after namespace if present.
        let mut emitted_imports = false;
        for d in &others {
            if matches!(d, Decl::Namespace(_)) {
                if !first {
                    self.newline();
                }
                first = false;
                self.decl(d);
                self.newline();
                if !imports.is_empty() {
                    self.newline();
                    for imp in &imports {
                        self.decl(imp);
                        self.newline();
                    }
                    emitted_imports = true;
                }
            } else {
                if !emitted_imports && !imports.is_empty() {
                    if !first {
                        self.newline();
                    }
                    for imp in &imports {
                        self.decl(imp);
                        self.newline();
                    }
                    emitted_imports = true;
                    first = false;
                }
                if !first {
                    self.newline();
                }
                first = false;
                self.decl(d);
                self.newline();
            }
        }
        if !emitted_imports {
            for imp in &imports {
                if !first {
                    self.newline();
                }
                first = false;
                self.decl(imp);
                self.newline();
            }
        }
    }

    fn decl(&mut self, d: &Decl) {
        let span = decl_span(d);
        self.emit_comments_before(span.start);
        match d {
            Decl::Namespace(n) => {
                self.attrs(&n.attrs);
                self.raw("namespace ");
                self.qual(&n.path);
            }
            Decl::Import(i) => {
                self.attrs(&i.attrs);
                self.raw("import ");
                self.qual(&i.path);
                if i.wildcard {
                    self.raw(".*");
                }
                if let Some(a) = &i.alias {
                    self.raw(" as ");
                    self.raw(a);
                }
            }
            Decl::Struct(s) => self.struct_like("struct", s),
            Decl::Class(c) => self.class_like(c),
            Decl::Enum(e) => self.enum_decl(e),
            Decl::Protocol(p) => self.protocol_decl(p),
            Decl::Extension(e) => self.extension_decl(e),
            Decl::Func(f) => self.func_decl(f),
            Decl::Let(l) => self.let_decl(l),
            Decl::Var(v) => self.var_decl(v),
            Decl::Const(c) => {
                if let Some(a) = c.access {
                    self.access(a);
                    self.space();
                }
                self.raw("const ");
                self.raw(&c.name);
                if let Some(ty) = &c.ty {
                    self.raw(": ");
                    self.ty(ty);
                }
                self.raw(" = ");
                self.expr(&c.value);
            }
            Decl::Attribute(a) => self.attr(a),
            Decl::CompileDeclare { name, value, .. } => {
                self.raw("#declare $");
                self.raw(name);
                self.space();
                self.expr(value);
            }
        }
    }

    fn struct_like(&mut self, kw: &str, s: &StructDecl) {
        self.attrs(&s.attrs);
        if let Some(a) = s.access {
            self.access(a);
            self.space();
        }
        self.raw(kw);
        self.space();
        self.raw(&s.name);
        self.generics(&s.generics);
        if !s.primary_ctor.is_empty() {
            self.raw("(");
            self.primary_params(&s.primary_ctor);
            self.raw(")");
        }
        if let Some(inh) = &s.inheritance {
            self.raw(": ");
            self.inheritance(inh);
        }
        self.where_clause(&s.where_clause);
        if s.members.is_empty() && s.primary_ctor.is_empty() {
            // bare type ok
        } else if !s.members.is_empty() {
            self.space();
            self.raw("{");
            self.newline();
            self.indent += 1;
            for m in &s.members {
                self.member(m);
                self.newline();
            }
            self.indent -= 1;
            self.raw("}");
        }
    }

    fn class_like(&mut self, c: &ClassDecl) {
        // Reuse struct printer shape via parallel fields
        self.attrs(&c.attrs);
        if let Some(a) = c.access {
            self.access(a);
            self.space();
        }
        self.raw(if c.is_actor { "actor " } else { "class " });
        self.raw(&c.name);
        self.generics(&c.generics);
        if !c.primary_ctor.is_empty() {
            self.raw("(");
            self.primary_params(&c.primary_ctor);
            self.raw(")");
        }
        if let Some(inh) = &c.inheritance {
            self.raw(": ");
            self.inheritance(inh);
        }
        self.where_clause(&c.where_clause);
        if !c.members.is_empty() {
            self.space();
            self.raw("{");
            self.newline();
            self.indent += 1;
            for m in &c.members {
                self.member(m);
                self.newline();
            }
            self.indent -= 1;
            self.raw("}");
        }
    }

    fn enum_decl(&mut self, e: &EnumDecl) {
        self.attrs(&e.attrs);
        if let Some(a) = e.access {
            self.access(a);
            self.space();
        }
        self.raw("enum ");
        self.raw(&e.name);
        self.generics(&e.generics);
        if let Some(inh) = &e.inheritance {
            self.raw(": ");
            self.inheritance(inh);
        }
        self.where_clause(&e.where_clause);
        self.space();
        self.raw("{");
        self.newline();
        self.indent += 1;
        for case in &e.cases {
            self.raw("case ");
            self.raw(&case.name);
            if !case.payload.is_empty() {
                self.raw("(");
                for (i, (n, t)) in case.payload.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    self.raw(n);
                    self.raw(": ");
                    self.ty(t);
                }
                self.raw(")");
            }
            if let Some(v) = &case.raw_value {
                self.raw(" = ");
                self.expr(v);
            }
            self.newline();
        }
        for m in &e.members {
            self.member(m);
            self.newline();
        }
        self.indent -= 1;
        self.raw("}");
    }

    fn protocol_decl(&mut self, p: &ProtocolDecl) {
        self.attrs(&p.attrs);
        if let Some(a) = p.access {
            self.access(a);
            self.space();
        }
        self.raw("protocol ");
        self.raw(&p.name);
        self.generics(&p.generics);
        if let Some(inh) = &p.inheritance {
            self.raw(": ");
            self.inheritance(inh);
        }
        self.where_clause(&p.where_clause);
        self.space();
        self.raw("{");
        self.newline();
        self.indent += 1;
        for m in &p.members {
            self.member(m);
            self.newline();
        }
        self.indent -= 1;
        self.raw("}");
    }

    fn extension_decl(&mut self, e: &ExtensionDecl) {
        self.attrs(&e.attrs);
        self.raw("extension ");
        self.ty(&e.extended_type);
        self.generics(&e.generics);
        if let Some(inh) = &e.inheritance {
            self.raw(": ");
            self.inheritance(inh);
        }
        self.where_clause(&e.where_clause);
        self.space();
        self.raw("{");
        self.newline();
        self.indent += 1;
        for m in &e.members {
            self.member(m);
            self.newline();
        }
        self.indent -= 1;
        self.raw("}");
    }

    fn member(&mut self, m: &MemberDecl) {
        match m {
            MemberDecl::Func(f) => self.func_decl(f),
            MemberDecl::Var(v) => self.var_decl(v),
            MemberDecl::Let(l) => self.let_decl(l),
            MemberDecl::Init(i) => {
                if let Some(a) = i.access {
                    self.access(a);
                    self.space();
                }
                self.raw("init");
                self.generics(&i.generics);
                self.raw("(");
                self.params(&i.params);
                self.raw(")");
                self.where_clause(&i.where_clause);
                self.throws(&i.throws);
                self.func_body(&i.body);
            }
            MemberDecl::Deinit(d) => {
                self.raw("deinit ");
                self.block(&d.body);
            }
            MemberDecl::Nested(d) => self.decl(d),
        }
    }

    fn func_decl(&mut self, f: &FuncDecl) {
        self.attrs(&f.attrs);
        if let Some(a) = f.access {
            self.access(a);
            self.space();
        }
        if f.is_async {
            self.raw("async ");
        }
        if f.is_override {
            self.raw("override ");
        }
        if f.is_virtual {
            self.raw("virtual ");
        }
        if f.is_abstract {
            self.raw("abstract ");
        }
        self.raw("func ");
        self.raw(&f.name);
        self.generics(&f.generics);
        self.raw("(");
        self.params(&f.params);
        self.raw(")");
        if let Some(rt) = &f.return_type {
            self.raw(" -> ");
            self.ty(rt);
        }
        self.where_clause(&f.where_clause);
        self.throws(&f.throws);
        self.func_body(&f.body);
    }

    fn func_body(&mut self, body: &FuncBody) {
        match body {
            FuncBody::Block(b) => {
                self.space();
                self.block(b);
            }
            FuncBody::Expr(e) => {
                self.raw(" => ");
                self.expr(e);
            }
            FuncBody::Missing => {}
        }
    }

    fn let_decl(&mut self, l: &LetDecl) {
        self.attrs(&l.attrs);
        if let Some(a) = l.access {
            self.access(a);
            self.space();
        }
        if l.is_async {
            self.raw("async ");
        }
        self.raw("let ");
        self.bindings(&l.bindings);
    }

    fn var_decl(&mut self, v: &VarDecl) {
        self.attrs(&v.attrs);
        if let Some(a) = v.access {
            self.access(a);
            self.space();
        }
        if let Some(s) = v.setter_access {
            self.access(s);
            self.raw("(set) ");
        }
        self.raw("var ");
        self.bindings(&v.bindings);
    }

    fn bindings(&mut self, bindings: &[PatternBinding]) {
        for (i, b) in bindings.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            self.pattern(&b.pattern);
            if let Some(ty) = &b.ty {
                self.raw(": ");
                self.ty(ty);
            }
            if let Some(init) = &b.initializer {
                self.raw(" = ");
                self.expr(init);
            }
        }
    }

    fn attrs(&mut self, attrs: &[Attribute]) {
        for a in attrs {
            self.attr(a);
            self.newline();
        }
    }

    fn attr(&mut self, a: &Attribute) {
        self.raw("@");
        self.raw(&a.name);
        if !a.args.is_empty() {
            self.raw("(");
            for (i, e) in a.args.iter().enumerate() {
                if i > 0 {
                    self.raw(", ");
                }
                self.expr(e);
            }
            self.raw(")");
        }
    }

    fn access(&mut self, a: Access) {
        self.raw(match a {
            Access::Open => "open",
            Access::Public => "public",
            Access::Internal => "internal",
            Access::Private => "private",
            Access::Protected => "protected",
            Access::Fileprivate => "fileprivate",
        });
    }

    fn qual(&mut self, q: &QualifiedName) {
        for (i, s) in q.segments.iter().enumerate() {
            if i > 0 {
                self.raw(".");
            }
            self.raw(s);
        }
    }

    fn generics(&mut self, g: &[GenericParam]) {
        if g.is_empty() {
            return;
        }
        self.raw("<");
        for (i, p) in g.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            self.raw(&p.name);
            if let Some(c) = &p.constraint {
                self.raw(": ");
                self.ty(c);
            }
        }
        self.raw(">");
    }

    fn where_clause(&mut self, w: &[GenericConstraint]) {
        if w.is_empty() {
            return;
        }
        self.raw(" where ");
        for (i, c) in w.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            self.ty(&c.subject);
            match &c.bound {
                GenericBound::Type(t) => {
                    self.raw(": ");
                    self.ty(t);
                }
                GenericBound::SameType(t) => {
                    self.raw(" == ");
                    self.ty(t);
                }
            }
        }
    }

    fn inheritance(&mut self, inh: &InheritanceClause) {
        for (i, t) in inh.items.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            self.ty(t);
        }
    }

    fn primary_params(&mut self, params: &[PrimaryParam]) {
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            if let Some(a) = p.access {
                self.access(a);
                self.space();
            }
            if let Some(s) = p.modifiers.storage {
                self.raw(match s {
                    StorageKind::Let => "let ",
                    StorageKind::Var => "var ",
                });
            }
            if p.modifiers.inout {
                self.raw("inout ");
            }
            self.raw(&p.name);
            self.raw(": ");
            self.ty(&p.ty);
            if let Some(d) = &p.default {
                self.raw(" = ");
                self.expr(d);
            }
        }
    }

    fn params(&mut self, params: &[Param]) {
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            if let Some(ext) = &p.external_name {
                self.raw(ext);
                self.space();
            }
            self.raw(&p.internal_name);
            self.raw(": ");
            self.ty(&p.ty);
            if let Some(d) = &p.default {
                self.raw(" = ");
                self.expr(d);
            }
        }
    }

    fn throws(&mut self, t: &ThrowsClause) {
        if t.types.is_empty() {
            return;
        }
        self.raw(" throws(");
        for (i, ty) in t.types.iter().enumerate() {
            if i > 0 {
                self.raw(", ");
            }
            self.ty(ty);
        }
        self.raw(")");
    }

    fn ty(&mut self, t: &Type) {
        match t {
            Type::Named { name, generics, .. } => {
                self.raw(name);
                if !generics.is_empty() {
                    self.raw("<");
                    for (i, g) in generics.iter().enumerate() {
                        if i > 0 {
                            self.raw(", ");
                        }
                        self.ty(g);
                    }
                    self.raw(">");
                }
            }
            Type::Optional { inner, .. } => {
                self.ty(inner);
                self.raw("?");
            }
            Type::Tuple { elements, .. } => {
                self.raw("(");
                for (i, (n, t)) in elements.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    if let Some(n) = n {
                        self.raw(n);
                        self.raw(": ");
                    }
                    self.ty(t);
                }
                self.raw(")");
            }
            Type::Function {
                params,
                return_type,
                throws,
                ..
            } => {
                self.raw("(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    self.ty(p);
                }
                self.raw(")");
                self.throws(throws);
                self.raw(" -> ");
                self.ty(return_type);
            }
            Type::Array { element, .. } => {
                self.raw("[");
                self.ty(element);
                self.raw("]");
            }
            Type::Dictionary { key, value, .. } => {
                self.raw("[");
                self.ty(key);
                self.raw(": ");
                self.ty(value);
                self.raw("]");
            }
            Type::Attributed {
                attribute, inner, ..
            } => {
                self.raw("@");
                self.raw(attribute);
                self.space();
                self.ty(inner);
            }
            Type::Error { .. } => self.raw("<error>"),
        }
    }

    fn block(&mut self, b: &Block) {
        self.raw("{");
        if b.stmts.is_empty() {
            self.raw("}");
            return;
        }
        self.newline();
        self.indent += 1;
        for s in &b.stmts {
            self.stmt(s);
            self.newline();
        }
        self.indent -= 1;
        self.raw("}");
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Expr { expr, .. } => self.expr(expr),
            Stmt::Let(l) => self.let_decl(l),
            Stmt::Var(v) => self.var_decl(v),
            Stmt::Return { value, .. } => {
                self.raw("return");
                if let Some(v) = value {
                    self.space();
                    self.expr(v);
                }
            }
            Stmt::Throw { value, .. } => {
                self.raw("throw ");
                self.expr(value);
            }
            Stmt::Break { label, .. } => {
                self.raw("break");
                if let Some(l) = label {
                    self.space();
                    self.raw(l);
                }
            }
            Stmt::Continue { label, .. } => {
                self.raw("continue");
                if let Some(l) = label {
                    self.space();
                    self.raw(l);
                }
            }
            Stmt::Guard {
                condition,
                else_body,
                ..
            } => {
                self.raw("guard ");
                match condition {
                    GuardCondition::Expr(e) => self.expr(e),
                    GuardCondition::Let(p, e) => {
                        self.raw("let ");
                        self.pattern(p);
                        self.raw(" = ");
                        self.expr(e);
                    }
                }
                self.raw(" else ");
                self.block(else_body);
            }
            Stmt::If {
                arms, else_branch, ..
            } => self.if_expr(arms, else_branch),
            Stmt::While {
                condition, body, ..
            } => {
                self.raw("while ");
                self.expr(condition);
                self.space();
                self.block(body);
            }
            Stmt::For {
                pattern,
                sequence,
                filter,
                body,
                ..
            } => {
                self.raw("for ");
                self.pattern(pattern);
                self.raw(" in ");
                self.expr(sequence);
                if let Some(f) = filter {
                    self.raw(" where ");
                    self.expr(f);
                }
                self.space();
                self.block(body);
            }
            Stmt::DoCatch { body, catches, .. } => {
                self.raw("do ");
                self.block(body);
                for c in catches {
                    self.space();
                    self.catch_clause(c);
                }
            }
            Stmt::Defer { body, .. } => {
                self.raw("#defer ");
                match body {
                    DeferBody::Block(b) => self.block(b),
                    DeferBody::Expr(e) => self.expr(e),
                }
            }
            Stmt::Die { message, .. } => {
                self.raw("#die ");
                self.raw(&format!("\"{message}\""));
            }
            Stmt::CompileIf {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.raw("#if ");
                self.expr(condition);
                self.space();
                self.block(then_body);
                if let Some(e) = else_body {
                    self.raw(" else ");
                    self.block(e);
                }
            }
            Stmt::Run { expr, .. } => {
                self.raw("@run ");
                self.expr(expr);
            }
            Stmt::Block(b) => self.block(b),
            Stmt::Error { .. } => self.raw("<error>"),
        }
    }

    fn if_expr(&mut self, arms: &[IfArm], else_branch: &Option<IfElse>) {
        for (i, arm) in arms.iter().enumerate() {
            if i == 0 {
                self.raw("if ");
            } else {
                self.raw(" elseif ");
            }
            match &arm.condition {
                IfCondition::Expr(e) => self.expr(e),
                IfCondition::Let(p, e) => {
                    self.raw("let ");
                    self.pattern(p);
                    if let Some(e) = e {
                        self.raw(" = ");
                        self.expr(e);
                    }
                }
                IfCondition::Var(p, e) => {
                    self.raw("var ");
                    self.pattern(p);
                    if let Some(e) = e {
                        self.raw(" = ");
                        self.expr(e);
                    }
                }
            }
            self.space();
            match &arm.body {
                IfBody::Block(b) => self.block(b),
                IfBody::Arrow(e) => {
                    self.raw("=> ");
                    self.expr(e);
                }
            }
        }
        if let Some(els) = else_branch {
            self.raw(" else ");
            match els {
                IfElse::Block(b) => self.block(b),
                IfElse::Expr(e) => self.expr(e),
                IfElse::If(e) => self.expr(e),
            }
        }
    }

    fn catch_clause(&mut self, c: &CatchClause) {
        self.raw("catch");
        if let Some(p) = &c.pattern {
            self.space();
            self.pattern(p);
        }
        self.space();
        match &c.body {
            CatchBody::Block(b) => self.block(b),
            CatchBody::Arrow(e) => {
                self.raw("=> ");
                self.expr(e);
            }
        }
    }

    fn pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Wildcard { .. } => self.raw("_"),
            Pattern::Ident { name, .. } => self.raw(name),
            Pattern::Typed { pattern, ty, .. } => {
                self.pattern(pattern);
                self.raw(": ");
                self.ty(ty);
            }
            Pattern::Tuple { elements, .. } => {
                self.raw("(");
                for (i, e) in elements.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    self.pattern(e);
                }
                self.raw(")");
            }
            Pattern::EnumCase {
                type_name,
                case,
                subpatterns,
                ..
            } => {
                if let Some(t) = type_name {
                    self.raw(t);
                    self.raw(".");
                }
                self.raw(case);
                if !subpatterns.is_empty() {
                    self.raw("(");
                    for (i, s) in subpatterns.iter().enumerate() {
                        if i > 0 {
                            self.raw(", ");
                        }
                        self.pattern(s);
                    }
                    self.raw(")");
                }
            }
            Pattern::Literal { expr, .. } => self.expr(expr),
            Pattern::Binding { kind, pattern, .. } => {
                self.raw(match kind {
                    StorageKind::Let => "let ",
                    StorageKind::Var => "var ",
                });
                self.pattern(pattern);
            }
            Pattern::Error { .. } => self.raw("<error>"),
        }
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::LiteralInt { value, .. } => self.raw(value),
            Expr::LiteralFloat { value, .. } => self.raw(value),
            Expr::LiteralBool { value, .. } => self.raw(if *value { "true" } else { "false" }),
            Expr::LiteralNil { .. } => self.raw("nil"),
            Expr::LiteralChar { value, .. } => {
                self.raw("'");
                self.raw(&value.escape_default().to_string());
                self.raw("'");
            }
            Expr::LiteralString { parts, .. } => self.string_lit(parts),
            Expr::Ident { name, .. } => self.raw(name),
            Expr::CompileConst { name, .. } => {
                self.raw("$");
                self.raw(name);
            }
            Expr::SelfExpr { kind, .. } => self.raw(match kind {
                SelfKind::Instance => "self",
                SelfKind::Type => "Self",
                SelfKind::Super => "super",
            }),
            Expr::Array { elements, .. } => {
                self.raw("[");
                for (i, el) in elements.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    self.expr(el);
                }
                self.raw("]");
            }
            Expr::Dictionary { entries, .. } => {
                self.raw("[");
                if entries.is_empty() {
                    self.raw(":");
                } else {
                    for (i, (k, v)) in entries.iter().enumerate() {
                        if i > 0 {
                            self.raw(", ");
                        }
                        self.expr(k);
                        self.raw(": ");
                        self.expr(v);
                    }
                }
                self.raw("]");
            }
            Expr::Unary { op, operand, .. } => {
                self.raw(unary_str(*op));
                self.expr(operand);
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                self.expr(left);
                self.raw(" ");
                self.raw(binary_str(*op));
                self.raw(" ");
                self.expr(right);
            }
            Expr::Call {
                callee,
                args,
                try_kind,
                ..
            } => {
                match try_kind {
                    TryCallKind::Try => self.raw("try "),
                    TryCallKind::TryOptional => self.raw("try? "),
                    TryCallKind::None => {}
                }
                self.expr(callee);
                self.raw("(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    if let Some(l) = &a.label {
                        self.raw(l);
                        self.raw(": ");
                    }
                    self.expr(&a.value);
                }
                self.raw(")");
            }
            Expr::Member {
                base,
                name,
                optional,
                ..
            } => {
                self.expr(base);
                self.raw(if *optional { "?." } else { "." });
                self.raw(name);
            }
            Expr::Index {
                base,
                indices,
                optional,
                ..
            } => {
                self.expr(base);
                if *optional {
                    self.raw("?");
                }
                self.raw("[");
                for (i, ix) in indices.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    self.expr(ix);
                }
                self.raw("]");
            }
            Expr::Assign {
                target, op, value, ..
            } => {
                self.expr(target);
                self.raw(" ");
                self.raw(assign_str(*op));
                self.raw(" ");
                self.expr(value);
            }
            Expr::If {
                arms, else_branch, ..
            } => self.if_expr(arms, else_branch),
            Expr::Switch {
                subject,
                arms,
                default,
                ..
            } => {
                self.raw("switch ");
                self.expr(subject);
                self.space();
                self.raw("{");
                self.newline();
                self.indent += 1;
                for arm in arms {
                    self.pattern(&arm.pattern);
                    if let Some(g) = &arm.guard {
                        self.raw(" where ");
                        self.expr(g);
                    }
                    match &arm.body {
                        SwitchBody::Arrow(e) => {
                            self.raw(" => ");
                            self.expr(e);
                        }
                        SwitchBody::Block(b) => {
                            self.space();
                            self.block(b);
                        }
                    }
                    self.newline();
                }
                if let Some(d) = default {
                    self.raw("default => ");
                    self.expr(d);
                    self.newline();
                }
                self.indent -= 1;
                self.raw("}");
            }
            Expr::Closure {
                params,
                return_type,
                body,
                ..
            } => {
                self.raw("{");
                if !params.is_empty() {
                    self.space();
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 {
                            self.raw(", ");
                        }
                        if let Some(n) = &p.name {
                            self.raw(n);
                        } else if let Some(pat) = &p.pattern {
                            self.pattern(pat);
                        }
                        if let Some(t) = &p.ty {
                            self.raw(": ");
                            self.ty(t);
                        }
                    }
                    if let Some(rt) = return_type {
                        self.raw(" -> ");
                        self.ty(rt);
                    }
                    self.raw(" in ");
                }
                match body {
                    ClosureBody::Expr(e) => self.expr(e),
                    ClosureBody::Block(b) => {
                        self.newline();
                        self.indent += 1;
                        for s in &b.stmts {
                            self.stmt(s);
                            self.newline();
                        }
                        self.indent -= 1;
                    }
                }
                self.raw("}");
            }
            Expr::Lambda { params, body, .. } => {
                self.raw("(");
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        self.raw(", ");
                    }
                    if let Some(n) = &p.name {
                        self.raw(n);
                    }
                }
                self.raw(") => ");
                self.expr(body);
            }
            Expr::Try {
                expr,
                optional,
                force,
                ..
            } => {
                self.raw(if *optional {
                    "try? "
                } else if *force {
                    "try! "
                } else {
                    "try "
                });
                self.expr(expr);
            }
            Expr::Await { expr, .. } => {
                self.raw("await ");
                self.expr(expr);
            }
            Expr::Cast { expr, ty, kind, .. } => {
                self.expr(expr);
                self.raw(match kind {
                    CastKind::As => " as ",
                    CastKind::AsOptional => " as? ",
                    CastKind::AsForced => " as! ",
                });
                self.ty(ty);
            }
            Expr::Is {
                expr,
                ty,
                binding,
                ..
            } => {
                self.expr(expr);
                self.raw(" is ");
                self.ty(ty);
                if let Some(name) = binding {
                    self.raw(" ");
                    self.raw(name);
                }
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.expr(condition);
                self.raw(" ? ");
                self.expr(then_expr);
                self.raw(" : ");
                self.expr(else_expr);
            }
            Expr::NilCoalesce { left, right, .. } => {
                self.expr(left);
                self.raw(" ?? ");
                self.expr(right);
            }
            Expr::ForceUnwrap { expr, .. } => {
                self.expr(expr);
                self.raw("!");
            }
            Expr::Range {
                start,
                end,
                inclusive,
                ..
            } => {
                self.expr(start);
                self.raw(if *inclusive { "..." } else { "..<" });
                self.expr(end);
            }
            Expr::DoCatch { body, catches, .. } => {
                self.raw("do ");
                self.block(body);
                for c in catches {
                    self.space();
                    self.catch_clause(c);
                }
            }
            Expr::Catch { expr, catches, .. } => {
                self.expr(expr);
                for c in catches {
                    self.space();
                    self.catch_clause(c);
                }
            }
            Expr::Unsafe { body, .. } => {
                self.raw("unsafe ");
                match body {
                    UnsafeBody::Block(b) => self.block(b),
                    UnsafeBody::Expr(e) => self.expr(e),
                }
            }
            Expr::Paren { expr, .. } => {
                self.raw("(");
                self.expr(expr);
                self.raw(")");
            }
            Expr::Error { .. } => self.raw("<error>"),
        }
        let _ = MAX_WIDTH; // reserved for soft-wrap pass
    }

    fn string_lit(&mut self, parts: &[StringPart]) {
        self.raw("\"");
        for part in parts {
            match part {
                StringPart::Text(t) => {
                    for ch in t.chars() {
                        match ch {
                            '\\' => self.raw("\\\\"),
                            '"' => self.raw("\\\""),
                            '\n' => self.raw("\\n"),
                            '\r' => self.raw("\\r"),
                            '\t' => self.raw("\\t"),
                            c => {
                                let mut buf = [0u8; 4];
                                self.raw(c.encode_utf8(&mut buf));
                            }
                        }
                    }
                }
                StringPart::Interpolation(e) => {
                    self.raw("\\(");
                    self.expr(e);
                    self.raw(")");
                }
            }
        }
        self.raw("\"");
    }
}

fn import_key(d: &Decl) -> String {
    match d {
        Decl::Import(i) => i.path.segments.join("."),
        _ => String::new(),
    }
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
        Decl::CompileDeclare { span, .. } => *span,
    }
}

fn unary_str(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
        UnaryOp::Pos => "+",
        UnaryOp::BitNot => "~",
        UnaryOp::Deref => "*",
        UnaryOp::Addr => "&",
    }
}

fn binary_str(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
}

fn assign_str(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Assign => "=",
        AssignOp::Add => "+=",
        AssignOp::Sub => "-=",
        AssignOp::Mul => "*=",
        AssignOp::Div => "/=",
        AssignOp::Mod => "%=",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hello() {
        let src = "namespace Hello\nimport Std.IO\npublic func main(){print(\"hi\")}";
        let out = format_source(src).expect("fmt");
        assert!(out.contains("public func main() {"));
        assert!(out.contains("print(\"hi\")"));
        let again = format_source(&out).expect("idempotent");
        assert_eq!(out, again);
    }

    #[test]
    fn preserves_line_comment() {
        let src = "// greeting\nnamespace Hello\n\npublic func main() {\n    print(\"x\")\n}\n";
        let out = format_source(src).expect("fmt");
        assert!(out.contains("// greeting"));
    }

    #[test]
    fn sorts_imports() {
        let src = "namespace N\nimport Std.Toon\nimport Std.Json\npublic func main() {}\n";
        let out = format_source(src).expect("fmt");
        let json = out.find("import Std.Json").expect("json");
        let toon = out.find("import Std.Toon").expect("toon");
        assert!(json < toon);
    }
}
