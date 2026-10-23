//! Recursive-descent parser with error recovery.

use crate::ast::*;
use crate::diagnostic::{codes, Diagnostic, Severity};
use crate::lexer::lex;
use crate::span::Span;
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
    source_len: u32,
}

impl Parser {
    pub fn parse_source(source: &str) -> (File, Vec<Diagnostic>) {
        let (tokens, lex_diags) = lex(source);
        let source_len = source.len() as u32;
        let mut parser = Parser {
            tokens,
            pos: 0,
            diagnostics: lex_diags,
            source_len,
        };
        let file = parser.parse_file();
        (file, parser.diagnostics)
    }

    fn parse_file(&mut self) -> File {
        let start = self.current_span().start;
        let mut decls = Vec::new();
        while !self.check_kind(&TokenKind::Eof) {
            if matches!(self.current_kind(), TokenKind::Semicolon) {
                self.bump();
                continue;
            }
            if !self.is_top_level_decl_start() {
                self.error_expected("top-level declaration", self.current_span());
                self.synchronize_top_level();
                continue;
            }
            match self.parse_top_level_decl() {
                Some(decl) => decls.push(decl),
                None => self.synchronize_top_level(),
            }
        }
        File {
            span: Span::new(start, self.source_len),
            decls,
        }
    }

    fn parse_top_level_decl(&mut self) -> Option<Decl> {
        if self.check_kind(&TokenKind::Hash) {
            if let Some(TokenKind::Ident(name)) = self.tokens.get(self.pos + 1).map(|t| &t.kind) {
                if name == "declare" {
                    return Some(self.parse_hash_declare_decl());
                }
            }
        }
        let mut attrs = self.parse_hash_decl_directives();
        attrs.extend(self.parse_attributes());
        let access = self.parse_access();
        if self.check_kind(&TokenKind::Namespace) {
            return Some(Decl::Namespace(self.parse_namespace(attrs)));
        }
        if self.check_kind(&TokenKind::Import) {
            return Some(Decl::Import(self.parse_import(attrs)));
        }
        if self.check_kind(&TokenKind::Extension) {
            return Some(Decl::Extension(self.parse_extension(attrs)));
        }
        let is_async = self.eat_kind(&TokenKind::Async);
        if self.check_kind(&TokenKind::Func) {
            return Some(Decl::Func(self.parse_func(attrs, access, is_async)));
        }
        if self.check_kind(&TokenKind::Let) {
            return Some(Decl::Let(self.parse_let_decl(attrs, access, is_async)));
        }
        if is_async {
            self.error_expected("`func` or `let` after `async`", self.current_span());
        }
        if self.check_kind(&TokenKind::Var) {
            return Some(Decl::Var(self.parse_var_decl(attrs, access, ParamModifiers::default())));
        }
        if self.check_kind(&TokenKind::Const) {
            return Some(Decl::Const(self.parse_const_decl(access)));
        }
        if self.check_kind(&TokenKind::Struct) {
            let d = self.parse_struct(attrs, access);
            return Some(Decl::Struct(d));
        }
        let is_abstract_type = self.eat_kind(&TokenKind::Abstract);
        let is_sealed_type = self.eat_kind(&TokenKind::Sealed);
        if self.check_kind(&TokenKind::Class) || self.check_kind(&TokenKind::Actor) {
            return Some(Decl::Class(self.parse_class(
                attrs,
                access,
                is_abstract_type,
                is_sealed_type,
            )));
        }
        if is_abstract_type || is_sealed_type {
            // Recover: modifiers without class/actor.
            self.error(
                "LL0001",
                "expected `class` or `actor` after abstract/sealed",
                self.current_span(),
            );
        }
        if self.check_kind(&TokenKind::Enum) {
            return Some(Decl::Enum(self.parse_enum(attrs, access)));
        }
        if self.check_kind(&TokenKind::Protocol) {
            return Some(Decl::Protocol(self.parse_protocol(attrs, access)));
        }
        if !attrs.is_empty() {
            if let Some(attr) = attrs.into_iter().next() {
                self.error_expected("declaration", self.current_span());
                return Some(Decl::Attribute(attr));
            }
        }
        if is_async {
            self.error_expected("func after async", self.current_span());
            return None;
        }
        if access.is_some() {
            self.error_expected("declaration after access modifier", self.current_span());
            self.synchronize_top_level();
            return None;
        }
        self.error_expected("top-level declaration", self.current_span());
        None
    }

    fn parse_namespace(&mut self, attrs: Vec<Attribute>) -> NamespaceDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Namespace);
        let path = self.parse_qualified_name();
        self.eat_kind(&TokenKind::Semicolon);
        NamespaceDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            path,
        }
    }

    fn parse_import(&mut self, attrs: Vec<Attribute>) -> ImportDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Import);
        let path = self.parse_qualified_name();
        let mut wildcard = false;
        if self.eat_kind(&TokenKind::Star)
            || (self.eat_kind(&TokenKind::Dot) && self.eat_kind(&TokenKind::Star))
        {
            wildcard = true;
        }
        let alias = if self.check_kind(&TokenKind::As) {
            self.bump();
            Some(self.parse_ident())
        } else {
            None
        };
        self.eat_kind(&TokenKind::Semicolon);
        ImportDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            path,
            alias,
            wildcard,
        }
    }

    fn parse_struct(&mut self, attrs: Vec<Attribute>, access: Option<Access>) -> StructDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Struct);
        let name = self.parse_ident();
        let generics = self.parse_generic_params();
        let primary_ctor = self.parse_primary_params();
        let inheritance = self.parse_inheritance();
        let where_clause = self.parse_where_clause();
        let members = self.parse_type_body();
        StructDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            name,
            generics,
            primary_ctor,
            inheritance,
            where_clause,
            members,
        }
    }

    fn parse_class(
        &mut self,
        attrs: Vec<Attribute>,
        access: Option<Access>,
        is_abstract: bool,
        is_sealed: bool,
    ) -> ClassDecl {
        let start = self.current_span().start;
        let is_actor = self.check_kind(&TokenKind::Actor);
        if is_actor {
            self.bump();
        } else {
            self.expect_kind(&TokenKind::Class);
        }
        let name = self.parse_ident();
        let generics = self.parse_generic_params();
        let primary_ctor = self.parse_primary_params();
        let inheritance = self.parse_inheritance();
        let where_clause = self.parse_where_clause();
        let members = self.parse_type_body();
        ClassDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            is_abstract,
            is_sealed,
            is_actor,
            name,
            generics,
            primary_ctor,
            inheritance,
            where_clause,
            members,
        }
    }

    fn parse_enum(&mut self, attrs: Vec<Attribute>, access: Option<Access>) -> EnumDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Enum);
        let name = self.parse_ident();
        let generics = self.parse_generic_params();
        let inheritance = self.parse_inheritance();
        let where_clause = self.parse_where_clause();
        let (cases, members) = self.parse_enum_body();
        EnumDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            name,
            generics,
            inheritance,
            where_clause,
            cases,
            members,
        }
    }

    fn parse_protocol(&mut self, attrs: Vec<Attribute>, access: Option<Access>) -> ProtocolDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Protocol);
        let name = self.parse_ident();
        let generics = self.parse_generic_params();
        let inheritance = self.parse_inheritance();
        let where_clause = self.parse_where_clause();
        let members = self.parse_type_body();
        ProtocolDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            name,
            generics,
            inheritance,
            where_clause,
            members,
        }
    }

    fn parse_extension(&mut self, attrs: Vec<Attribute>) -> ExtensionDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Extension);
        let extended_type = self.parse_type();
        let generics = self.parse_generic_params();
        let inheritance = self.parse_inheritance();
        let where_clause = self.parse_where_clause();
        let members = self.parse_type_body();
        ExtensionDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            extended_type,
            generics,
            inheritance,
            where_clause,
            members,
        }
    }

    fn parse_enum_body(&mut self) -> (Vec<EnumCase>, Vec<MemberDecl>) {
        let mut cases = Vec::new();
        let mut members = Vec::new();
        if !self.expect_kind(&TokenKind::LBrace) {
            return (cases, members);
        }
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if self.current_text() == "case" {
                self.error(
                    codes::UNEXPECTED_TOKEN,
                    "enum cases do not use the `case` keyword",
                    self.current_span(),
                );
                self.bump();
                continue;
            }
            if self.is_member_start() {
                if let Some(m) = self.parse_member_decl() {
                    members.push(m);
                } else {
                    self.synchronize_member();
                }
                continue;
            }
            if self.check_ident_start() {
                cases.push(self.parse_enum_case());
                self.eat_kind(&TokenKind::Comma);
                continue;
            }
            self.error_expected("enum case or member", self.current_span());
            self.bump();
        }
        self.eat_kind(&TokenKind::RBrace);
        (cases, members)
    }

    fn parse_enum_case(&mut self) -> EnumCase {
        let start = self.current_span().start;
        let name = self.parse_ident();
        let mut payload = Vec::new();
        if self.eat_kind(&TokenKind::LParen) {
            while !self.check_kind(&TokenKind::RParen) && !self.check_kind(&TokenKind::Eof) {
                // Named payload `label: Type` vs anonymous `Type`
                if self.check_ident_start() && self.peek_kind_is(1, &TokenKind::Colon) {
                    let pname = self.parse_ident();
                    self.expect_kind(&TokenKind::Colon);
                    let ty = self.parse_type();
                    payload.push((pname, ty));
                } else {
                    let ty = self.parse_type();
                    payload.push((String::new(), ty));
                }
                if !self.eat_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(&TokenKind::RParen);
        }
        let raw_value = if self.eat_kind(&TokenKind::Assign) {
            Some(self.parse_expr())
        } else {
            None
        };
        EnumCase {
            span: span_from(start, self.prev_span()),
            name,
            payload,
            raw_value,
        }
    }

    fn parse_type_body(&mut self) -> Vec<MemberDecl> {
        let mut members = Vec::new();
        // Primary-constructor-only types may omit `{ }` entirely.
        if !self.check_kind(&TokenKind::LBrace) {
            return members;
        }
        self.bump();
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if let Some(m) = self.parse_member_decl() {
                members.push(m);
            } else {
                self.synchronize_member();
            }
        }
        self.eat_kind(&TokenKind::RBrace);
        members
    }

    fn parse_member_decl(&mut self) -> Option<MemberDecl> {
        let mut attrs = self.parse_hash_decl_directives();
        attrs.extend(self.parse_attributes());
        let access = self.parse_access();
        if self.check_kind(&TokenKind::Deinit) {
            let start = self.current_span().start;
            self.bump();
            let body = self.parse_block();
            return Some(MemberDecl::Deinit(DeinitDecl {
                span: span_from(start, self.prev_span()),
                body,
            }));
        }
        if self.check_kind(&TokenKind::Init) {
            return Some(MemberDecl::Init(self.parse_init(attrs, access)));
        }
        let is_async = self.eat_kind(&TokenKind::Async);
        let is_override = self.eat_kind(&TokenKind::Override);
        let is_virtual = self.eat_kind(&TokenKind::Virtual);
        let is_abstract = self.eat_kind(&TokenKind::Abstract);
        let is_sealed = self.eat_kind(&TokenKind::Sealed);
        if self.check_kind(&TokenKind::Func) {
            let mut f = self.parse_func(attrs, access, is_async);
            f.is_override = f.is_override || is_override;
            f.is_virtual = f.is_virtual || is_virtual;
            f.is_abstract = f.is_abstract || is_abstract;
            f.is_sealed = f.is_sealed || is_sealed;
            return Some(MemberDecl::Func(f));
        }
        // `public weak var` / `unowned let` — modifiers precede storage keyword.
        // `public private(set) var` — setter narrower than getter (ADR 009).
        let setter_access = self.parse_setter_access();
        let mods = self.parse_var_modifiers();
        if self.check_kind(&TokenKind::Let) {
            return Some(MemberDecl::Let(self.parse_let_decl(attrs, access, is_async)));
        }
        if self.check_kind(&TokenKind::Var) {
            let mut v = self.parse_var_decl(attrs, access, mods);
            v.setter_access = setter_access;
            return Some(MemberDecl::Var(v));
        }
        if self.check_kind(&TokenKind::Struct)
            || self.check_kind(&TokenKind::Class)
            || self.check_kind(&TokenKind::Actor)
            || self.check_kind(&TokenKind::Enum)
            || self.check_kind(&TokenKind::Protocol)
        {
            if let Some(d) = self.parse_top_level_decl() {
                return Some(MemberDecl::Nested(Box::new(d)));
            }
        }
        // Custom `subscript` / `operator` — grammar tokens exist; declarations are Later (ADR/ROADMAP).
        if self.check_kind(&TokenKind::Subscript) || self.check_kind(&TokenKind::Operator) {
            let span = self.current_span();
            let kind = if self.check_kind(&TokenKind::Subscript) {
                "subscript"
            } else {
                "operator"
            };
            self.bump();
            self.error(
                "LL3001",
                format!(
                    "custom `{kind}` declarations are not implemented yet (deferred to Later)"
                ),
                span,
            );
            // Skip until next member-ish boundary.
            while !self.check_kind(&TokenKind::Eof)
                && !self.check_kind(&TokenKind::RBrace)
                && !self.check_kind(&TokenKind::Func)
                && !self.check_kind(&TokenKind::Var)
                && !self.check_kind(&TokenKind::Let)
            {
                if self.check_kind(&TokenKind::LBrace) {
                    let _ = self.parse_block();
                    break;
                }
                self.bump();
            }
            return None;
        }
        if !attrs.is_empty() || access.is_some() || is_async || is_override || is_virtual || is_abstract {
            self.error_expected("member declaration", self.current_span());
        }
        None
    }

    fn parse_init(&mut self, _attrs: Vec<Attribute>, access: Option<Access>) -> InitDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Init);
        let generics = self.parse_generic_params();
        let params = self.parse_param_list();
        let where_clause = self.parse_where_clause();
        let throws = self.parse_throws_clause();
        let body = self.parse_func_body();
        InitDecl {
            span: span_from(start, self.prev_span()),
            access,
            generics,
            params,
            where_clause,
            throws,
            body,
        }
    }

    fn parse_func(
        &mut self,
        attrs: Vec<Attribute>,
        access: Option<Access>,
        is_async: bool,
    ) -> FuncDecl {
        let start = self.current_span().start;
        let is_override = self.eat_kind(&TokenKind::Override);
        let is_virtual = self.eat_kind(&TokenKind::Virtual);
        let is_abstract = self.eat_kind(&TokenKind::Abstract);
        let is_sealed = self.eat_kind(&TokenKind::Sealed);
        self.expect_kind(&TokenKind::Func);
        let name = self.parse_ident();
        let generics = self.parse_generic_params();
        let params = self.parse_param_list();
        // Allow `func f() async throws` (async after parameter list).
        let is_async = is_async || self.eat_kind(&TokenKind::Async);
        let where_clause = self.parse_where_clause();
        let (return_type, throws) = self.parse_return_type_and_throws();
        let body = self.parse_func_body();
        FuncDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            is_async,
            is_override,
            is_virtual,
            is_abstract,
            is_sealed,
            name,
            generics,
            params,
            return_type,
            where_clause,
            throws,
            body,
        }
    }

    fn parse_func_body(&mut self) -> FuncBody {
        if self.eat_kind(&TokenKind::FatArrow) {
            let expr = self.parse_expr();
            self.eat_kind(&TokenKind::Semicolon);
            return FuncBody::Expr(Box::new(expr));
        }
        if self.check_kind(&TokenKind::LBrace) {
            return FuncBody::Block(self.parse_block());
        }
        // Protocol requirements / `.sbl` declarations may omit bodies.
        FuncBody::Missing
    }

    fn parse_let_decl(
        &mut self,
        attrs: Vec<Attribute>,
        access: Option<Access>,
        is_async: bool,
    ) -> LetDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Let);
        let bindings = self.parse_pattern_bindings();
        self.eat_kind(&TokenKind::Semicolon);
        LetDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            is_async,
            bindings,
        }
    }

    fn parse_var_decl(
        &mut self,
        attrs: Vec<Attribute>,
        access: Option<Access>,
        modifiers: ParamModifiers,
    ) -> VarDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Var);
        let bindings = self.parse_pattern_bindings();
        let mut computed = None;
        let mut will_set = None;
        let mut did_set = None;
        // Computed: `=> expr` or `{ get … set … }` when no stored initializer.
        let has_init = bindings.iter().any(|b| b.initializer.is_some());
        if !has_init && bindings.len() == 1 {
            if self.eat_kind(&TokenKind::FatArrow) {
                computed = Some(ComputedBody::Arrow(Box::new(self.parse_expr())));
            } else if self.check_kind(&TokenKind::LBrace)
                && (self.peek_kind_is(1, &TokenKind::Get)
                    || self.peek_kind_is(1, &TokenKind::Set)
                    || self.peek_kind_is(1, &TokenKind::WillSet)
                    || self.peek_kind_is(1, &TokenKind::DidSet))
            {
                computed = Some(self.parse_computed_accessors());
            }
        }
        if computed.is_none() {
            will_set = self.parse_property_observer(TokenKind::WillSet);
            did_set = self.parse_property_observer(TokenKind::DidSet);
        }
        self.eat_kind(&TokenKind::Semicolon);
        VarDecl {
            span: span_from(start, self.prev_span()),
            attrs,
            access,
            setter_access: None,
            modifiers,
            bindings,
            computed,
            will_set,
            did_set,
        }
    }

    fn parse_property_observer(&mut self, kind: TokenKind) -> Option<PropertyObserver> {
        if !self.check_kind(&kind) {
            return None;
        }
        let start = self.current_span().start;
        self.bump();
        let param_name = if self.eat_kind(&TokenKind::LParen) {
            let n = self.parse_ident();
            self.expect_kind(&TokenKind::RParen);
            Some(n)
        } else {
            None
        };
        let body = self.parse_block();
        Some(PropertyObserver {
            span: span_from(start, self.prev_span()),
            param_name,
            body,
        })
    }

    fn parse_computed_accessors(&mut self) -> ComputedBody {
        self.expect_kind(&TokenKind::LBrace);
        let mut getter = None;
        let mut setter = None;
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if self.eat_kind(&TokenKind::Get) {
                getter = Some(self.parse_block());
            } else if self.eat_kind(&TokenKind::Set) {
                let param = if self.eat_kind(&TokenKind::LParen) {
                    let n = self.parse_ident();
                    self.expect_kind(&TokenKind::RParen);
                    Some(n)
                } else {
                    None
                };
                let body = self.parse_block();
                setter = Some((param, body));
            } else {
                self.error_expected("`get` or `set`", self.current_span());
                self.bump();
            }
        }
        self.expect_kind(&TokenKind::RBrace);
        ComputedBody::Accessors {
            getter: getter.unwrap_or_else(|| Block {
                span: Span::EMPTY,
                stmts: vec![],
            }),
            setter,
        }
    }

    fn parse_const_decl(&mut self, access: Option<Access>) -> ConstDecl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Const);
        let name = self.parse_ident();
        let ty = if self.eat_kind(&TokenKind::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect_kind(&TokenKind::Assign);
        let value = self.parse_expr();
        self.eat_kind(&TokenKind::Semicolon);
        ConstDecl {
            span: span_from(start, self.prev_span()),
            access,
            name,
            ty,
            value,
        }
    }

    fn parse_pattern_bindings(&mut self) -> Vec<PatternBinding> {
        let mut bindings = Vec::new();
        loop {
            bindings.push(self.parse_pattern_binding());
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        bindings
    }

    fn parse_pattern_binding(&mut self) -> PatternBinding {
        let start = self.current_span().start;
        let pattern = self.parse_pattern();
        let ty = if self.eat_kind(&TokenKind::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        let initializer = if self.eat_kind(&TokenKind::Assign) {
            Some(self.parse_expr())
        } else {
            None
        };
        PatternBinding {
            span: span_from(start, self.prev_span()),
            pattern,
            ty,
            initializer,
        }
    }

    fn parse_pattern(&mut self) -> Pattern {
        let start = self.current_span().start;
        if self.check_ident_start() && self.current_text() == "_" {
            self.bump();
            return Pattern::Wildcard {
                span: span_from(start, self.prev_span()),
            };
        }
        if self.check_kind(&TokenKind::Let) || self.check_kind(&TokenKind::Var) {
            let kind = if self.eat_kind(&TokenKind::Let) {
                StorageKind::Let
            } else {
                self.bump();
                StorageKind::Var
            };
            let inner = self.parse_pattern();
            return Pattern::Binding {
                span: span_from(start, self.prev_span()),
                kind,
                pattern: Box::new(inner),
            };
        }
        if self.eat_kind(&TokenKind::LParen) {
            let mut elements = Vec::new();
            while !self.check_kind(&TokenKind::RParen) && !self.check_kind(&TokenKind::Eof) {
                elements.push(self.parse_pattern());
                if !self.eat_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(&TokenKind::RParen);
            return Pattern::Tuple {
                span: span_from(start, self.prev_span()),
                elements,
            };
        }
        if self.eat_kind(&TokenKind::Dot) {
            let case = self.parse_ident();
            let mut subpatterns = Vec::new();
            if self.eat_kind(&TokenKind::LParen) {
                while !self.check_kind(&TokenKind::RParen) {
                    subpatterns.push(self.parse_pattern());
                    if !self.eat_kind(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect_kind(&TokenKind::RParen);
            }
            return Pattern::EnumCase {
                span: span_from(start, self.prev_span()),
                type_name: None,
                case,
                subpatterns,
            };
        }
        if self.is_literal_start() {
            let expr = self.parse_primary_expr();
            return Pattern::Literal {
                span: span_from(start, self.prev_span()),
                expr: Box::new(expr),
            };
        }
        if self.check_ident_start() {
            let name = self.parse_ident();
            if self.eat_kind(&TokenKind::Colon) {
                let ty = self.parse_type();
                return Pattern::Typed {
                    span: span_from(start, self.prev_span()),
                    pattern: Box::new(Pattern::Ident {
                        span: span_from(start, self.prev_span()),
                        name,
                    }),
                    ty,
                };
            }
            return Pattern::Ident {
                span: span_from(start, self.prev_span()),
                name,
            };
        }
        Pattern::Error {
            span: self.current_span(),
        }
    }

    fn parse_block(&mut self) -> Block {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::LBrace);
        let mut stmts = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.synchronize_stmt();
            }
        }
        self.eat_kind(&TokenKind::RBrace);
        Block {
            span: span_from(start, self.prev_span()),
            stmts,
        }
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        if self.check_kind(&TokenKind::RBrace) || self.check_kind(&TokenKind::Eof) {
            return None;
        }
        let start = self.current_span().start;
        let stmt_async = self.eat_kind(&TokenKind::Async);
        if self.check_kind(&TokenKind::Let) {
            let decl = self.parse_let_decl(Vec::new(), None, stmt_async);
            return Some(Stmt::Let(decl));
        }
        if stmt_async {
            self.error_expected("`let` after `async` in statement position", self.current_span());
            return None;
        }
        if self.check_kind(&TokenKind::Var) {
            let mods = self.parse_var_modifiers();
            let decl = self.parse_var_decl(Vec::new(), None, mods);
            return Some(Stmt::Var(decl));
        }
        if self.check_kind(&TokenKind::Return) {
            self.bump();
            let value = if self.is_expr_start() {
                Some(self.parse_expr())
            } else {
                None
            };
            self.eat_kind(&TokenKind::Semicolon);
            return Some(Stmt::Return {
                span: span_from(start, self.prev_span()),
                value,
            });
        }
        if self.check_kind(&TokenKind::Throw) {
            self.bump();
            let value = self.parse_expr();
            self.eat_kind(&TokenKind::Semicolon);
            return Some(Stmt::Throw {
                span: span_from(start, self.prev_span()),
                value,
            });
        }
        if self.check_kind(&TokenKind::Break) {
            self.bump();
            let label = if self.check_ident_start() {
                Some(self.parse_ident())
            } else {
                None
            };
            self.eat_kind(&TokenKind::Semicolon);
            return Some(Stmt::Break {
                span: span_from(start, self.prev_span()),
                label,
            });
        }
        if self.check_kind(&TokenKind::Continue) {
            self.bump();
            let label = if self.check_ident_start() {
                Some(self.parse_ident())
            } else {
                None
            };
            self.eat_kind(&TokenKind::Semicolon);
            return Some(Stmt::Continue {
                span: span_from(start, self.prev_span()),
                label,
            });
        }
        if self.check_kind(&TokenKind::Guard) {
            return Some(self.parse_guard_stmt(start));
        }
        if self.check_kind(&TokenKind::If) {
            return Some(self.parse_if_stmt(start));
        }
        if self.check_kind(&TokenKind::While) {
            self.bump();
            let condition = self.parse_expr();
            let body = self.parse_block();
            return Some(Stmt::While {
                span: span_from(start, self.prev_span()),
                label: None,
                condition,
                body,
            });
        }
        if self.check_kind(&TokenKind::For) {
            return Some(self.parse_for_stmt(start));
        }
        if self.check_kind(&TokenKind::Do) {
            return Some(self.parse_do_stmt(start));
        }
        if self.check_kind(&TokenKind::Defer) {
            self.bump();
            let body = DeferBody::Block(self.parse_block());
            self.eat_kind(&TokenKind::Semicolon);
            return Some(Stmt::Defer {
                span: span_from(start, self.prev_span()),
                body,
            });
        }
        if self.check_kind(&TokenKind::Hash) {
            return self.parse_hash_directive(start);
        }
        // `@run expr` fire-and-forget (Phase 10).
        if self.check_kind(&TokenKind::At) {
            let attrs = self.parse_attributes();
            if attrs.iter().any(|a| a.name == "run") {
                let expr = self.parse_expr();
                self.eat_kind(&TokenKind::Semicolon);
                return Some(Stmt::Run {
                    span: span_from(start, self.prev_span()),
                    expr,
                });
            }
            // Unknown @attr at statement position — fall through as error recovery.
            self.error_expected("statement after attribute", self.current_span());
            return None;
        }
        if self.check_kind(&TokenKind::LBrace) {
            return Some(Stmt::Block(self.parse_block()));
        }
        if self.is_expr_start() {
            let expr = self.parse_expr();
            self.eat_kind(&TokenKind::Semicolon);
            return Some(Stmt::Expr {
                span: span_from(start, self.prev_span()),
                expr,
            });
        }
        None
    }

    fn parse_hash_directive(&mut self, start: u32) -> Option<Stmt> {
        self.expect_kind(&TokenKind::Hash);
        // Allow `# defer` with whitespace (Ident) or glued `#defer` as Ident after Hash.
        // `#if` uses the `If` keyword token.
        let dir = if let TokenKind::Ident(name) = self.current_kind().clone() {
            self.bump();
            name
        } else if self.check_kind(&TokenKind::Defer) {
            self.bump();
            "defer".into()
        } else if self.check_kind(&TokenKind::If) {
            self.bump();
            "if".into()
        } else {
            self.error_expected("directive name after `#`", self.current_span());
            return None;
        };
        match dir.as_str() {
            "defer" => {
                let body = if self.check_kind(&TokenKind::LBrace) {
                    DeferBody::Block(self.parse_block())
                } else {
                    let expr = self.parse_expr();
                    self.eat_kind(&TokenKind::Semicolon);
                    DeferBody::Expr(expr)
                };
                Some(Stmt::Defer {
                    span: span_from(start, self.prev_span()),
                    body,
                })
            }
            "die" => {
                let message = if let TokenKind::String(s) = self.current_kind().clone() {
                    self.bump();
                    s
                } else {
                    self.error_expected("string message after `#die`", self.current_span());
                    String::new()
                };
                self.eat_kind(&TokenKind::Semicolon);
                Some(Stmt::Die {
                    span: span_from(start, self.prev_span()),
                    message,
                })
            }
            "if" => Some(self.parse_hash_if(start)),
            "declare" => {
                self.error(
                    codes::UNEXPECTED_TOKEN,
                    "`#declare` is only allowed at file scope",
                    span_from(start, self.current_span()),
                );
                // Consume `$NAME lit` to recover.
                let _ = self.parse_hash_declare_after_hash(start);
                None
            }
            other => {
                self.error(
                    codes::UNEXPECTED_TOKEN,
                    format!("unknown directive `#{other}`"),
                    span_from(start, self.prev_span()),
                );
                None
            }
        }
    }

    fn parse_hash_if(&mut self, start: u32) -> Stmt {
        // `#` and `if` already consumed by caller... wait, caller already bumped both.
        let condition = self.parse_compile_const_or_expr();
        let then_body = self.parse_block();
        let else_body = if self.eat_kind(&TokenKind::Else) {
            Some(self.parse_block())
        } else {
            None
        };
        Stmt::CompileIf {
            span: span_from(start, self.prev_span()),
            condition,
            then_body,
            else_body,
        }
    }

    fn parse_compile_const_or_expr(&mut self) -> Expr {
        if let TokenKind::DollarIdent(name) = self.current_kind().clone() {
            let span = self.current_span();
            self.bump();
            return Expr::CompileConst { span, name };
        }
        self.parse_expr()
    }

    fn parse_hash_declare_decl(&mut self) -> Decl {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Hash);
        self.parse_hash_declare_after_hash(start)
    }

    fn parse_hash_declare_after_hash(&mut self, start: u32) -> Decl {
        // Expect `declare` (already may be current after Hash bumped in directive path).
        if let TokenKind::Ident(n) = self.current_kind().clone() {
            if n == "declare" {
                self.bump();
            }
        }
        let name = if let TokenKind::DollarIdent(n) = self.current_kind().clone() {
            self.bump();
            n
        } else {
            self.error_expected("`$NAME` after `#declare`", self.current_span());
            String::new()
        };
        let value = if let Some(lit) = self.parse_literal() {
            lit
        } else {
            self.error_expected("literal after `#declare $NAME`", self.current_span());
            Expr::Error {
                span: self.current_span(),
            }
        };
        self.eat_kind(&TokenKind::Semicolon);
        Decl::CompileDeclare {
            span: span_from(start, self.prev_span()),
            name,
            value,
        }
    }

    fn parse_guard_stmt(&mut self, start: u32) -> Stmt {
        self.expect_kind(&TokenKind::Guard);
        let condition = if self.check_kind(&TokenKind::Let) {
            self.bump();
            let pat = self.parse_pattern();
            self.expect_kind(&TokenKind::Assign);
            let expr = self.parse_expr();
            GuardCondition::Let(pat, expr)
        } else {
            GuardCondition::Expr(self.parse_expr())
        };
        self.expect_kind(&TokenKind::Else);
        let else_body = self.parse_block();
        self.eat_kind(&TokenKind::Semicolon);
        Stmt::Guard {
            span: span_from(start, self.prev_span()),
            condition,
            else_body,
        }
    }

    fn parse_if_stmt(&mut self, start: u32) -> Stmt {
        let (arms, else_branch) = self.parse_if_expr_parts();
        Stmt::If {
            span: span_from(start, self.prev_span()),
            arms,
            else_branch,
        }
    }

    fn parse_for_stmt(&mut self, start: u32) -> Stmt {
        self.expect_kind(&TokenKind::For);
        let pattern = self.parse_pattern();
        self.expect_kind(&TokenKind::In);
        let sequence = self.parse_expr();
        let filter = if self.eat_kind(&TokenKind::Where) {
            Some(self.parse_expr())
        } else {
            None
        };
        let body = self.parse_block();
        Stmt::For {
            span: span_from(start, self.prev_span()),
            label: None,
            pattern,
            sequence,
            filter,
            body,
        }
    }

    fn parse_do_stmt(&mut self, start: u32) -> Stmt {
        self.expect_kind(&TokenKind::Do);
        let body = self.parse_block();
        let mut catches = Vec::new();
        while self.check_kind(&TokenKind::Catch) {
            catches.push(self.parse_catch_clause());
        }
        Stmt::DoCatch {
            span: span_from(start, self.prev_span()),
            body,
            catches,
        }
    }

    fn parse_catch_clause(&mut self) -> CatchClause {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Catch);
        let pattern = if self.is_pattern_start() {
            Some(self.parse_pattern())
        } else {
            None
        };
        let body = if self.eat_kind(&TokenKind::FatArrow) {
            CatchBody::Arrow(Box::new(self.parse_expr()))
        } else {
            CatchBody::Block(self.parse_block())
        };
        CatchClause {
            span: span_from(start, self.prev_span()),
            pattern,
            body,
        }
    }

    fn parse_type(&mut self) -> Type {
        let atom = self.parse_type_atom();
        self.parse_type_suffix(atom)
    }

    fn parse_type_atom(&mut self) -> Type {
        let start = self.current_span().start;
        if self.eat_kind(&TokenKind::At) {
            let attr = self.parse_ident();
            let inner = self.parse_type_atom();
            return Type::Attributed {
                span: span_from(start, self.prev_span()),
                attribute: attr,
                inner: Box::new(inner),
            };
        }
        if self.eat_kind(&TokenKind::LParen) {
            let mut elements = Vec::new();
            if !self.check_kind(&TokenKind::RParen) {
                loop {
                    let name = if self.check_ident_start() {
                        let id = self.parse_ident();
                        if self.eat_kind(&TokenKind::Colon) {
                            Some(id)
                        } else {
                            self.pos -= 1;
                            None
                        }
                    } else {
                        None
                    };
                    let ty = self.parse_type();
                    elements.push((name, ty));
                    if !self.eat_kind(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect_kind(&TokenKind::RParen);
            if self.eat_kind(&TokenKind::Arrow) {
                let ret = self.parse_type();
                let throws = self.parse_throws_clause();
                return Type::Function {
                    span: span_from(start, self.prev_span()),
                    params: elements.into_iter().map(|(_, t)| t).collect(),
                    return_type: Box::new(ret),
                    throws,
                };
            }
            return Type::Tuple {
                span: span_from(start, self.prev_span()),
                elements,
            };
        }
        if self.eat_kind(&TokenKind::LBracket) {
            let element = self.parse_type();
            self.expect_kind(&TokenKind::RBracket);
            return Type::Array {
                span: span_from(start, self.prev_span()),
                element: Box::new(element),
            };
        }
        if self.check_ident_start() {
            let name = self.parse_ident();
            let generics = self.parse_type_generic_args();
            return Type::Named {
                span: span_from(start, self.prev_span()),
                name,
                generics,
            };
        }
        Type::Error {
            span: self.current_span(),
        }
    }

    fn parse_type_suffix(&mut self, mut ty: Type) -> Type {
        if self.eat_kind(&TokenKind::Question) {
            let span = ty.span().merge(self.prev_span());
            ty = Type::Optional {
                span,
                inner: Box::new(ty),
            };
        }
        ty
    }

    fn parse_type_generic_args(&mut self) -> Vec<Type> {
        if !self.eat_kind(&TokenKind::Lt) {
            return Vec::new();
        }
        let mut args = Vec::new();
        while !self.check_kind(&TokenKind::Gt) && !self.check_kind(&TokenKind::Eof) {
            args.push(self.parse_type());
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        if !self.eat_kind(&TokenKind::Gt) {
            self.diagnostic(codes::UNCLOSED_GENERIC, "expected `>`", self.current_span());
        }
        args
    }

    fn parse_generic_params(&mut self) -> Vec<GenericParam> {
        if !self.eat_kind(&TokenKind::Lt) {
            return Vec::new();
        }
        let mut params = Vec::new();
        while !self.check_kind(&TokenKind::Gt) && !self.check_kind(&TokenKind::Eof) {
            let start = self.current_span().start;
            let name = self.parse_ident();
            let constraint = if self.eat_kind(&TokenKind::Colon) {
                Some(self.parse_type())
            } else {
                None
            };
            params.push(GenericParam {
                span: span_from(start, self.prev_span()),
                name,
                constraint,
            });
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        if !self.eat_kind(&TokenKind::Gt) {
            self.diagnostic(codes::UNCLOSED_GENERIC, "expected `>`", self.current_span());
        }
        params
    }

    fn parse_primary_params(&mut self) -> Vec<PrimaryParam> {
        if !self.eat_kind(&TokenKind::LParen) {
            return Vec::new();
        }
        let mut params = Vec::new();
        while !self.check_kind(&TokenKind::RParen) && !self.check_kind(&TokenKind::Eof) {
            params.push(self.parse_primary_param());
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(&TokenKind::RParen);
        params
    }

    fn parse_primary_param(&mut self) -> PrimaryParam {
        let start = self.current_span().start;
        let access = self.parse_access();
        let mut modifiers = self.parse_param_modifiers();
        if modifiers.storage.is_none() {
            if self.eat_kind(&TokenKind::Let) {
                modifiers.storage = Some(StorageKind::Let);
            } else if self.eat_kind(&TokenKind::Var) {
                modifiers.storage = Some(StorageKind::Var);
            }
        }
        let name = self.parse_ident();
        self.expect_kind(&TokenKind::Colon);
        let ty = self.parse_type();
        let default = if self.eat_kind(&TokenKind::Assign) {
            Some(self.parse_expr())
        } else {
            None
        };
        PrimaryParam {
            span: span_from(start, self.prev_span()),
            access,
            modifiers,
            name,
            ty,
            default,
        }
    }

    fn parse_param_list(&mut self) -> Vec<Param> {
        if !self.eat_kind(&TokenKind::LParen) {
            return Vec::new();
        }
        let mut params = Vec::new();
        while !self.check_kind(&TokenKind::RParen) && !self.check_kind(&TokenKind::Eof) {
            params.push(self.parse_param());
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(&TokenKind::RParen);
        params
    }

    fn parse_param(&mut self) -> Param {
        let start = self.current_span().start;
        let external = if self.check_ident_start() && self.current_text() == "_" {
            self.bump();
            Some("_".to_string())
        } else if self.check_ident_start() {
            let id = self.parse_ident();
            if self.check_kind(&TokenKind::Colon) {
                Some(id)
            } else {
                self.pos -= 1;
                None
            }
        } else {
            None
        };
        let internal = if self.check_ident_start() {
            self.parse_ident()
        } else {
            external.clone().unwrap_or_else(|| "_".to_string())
        };
        self.expect_kind(&TokenKind::Colon);
        let ty = self.parse_type();
        let default = if self.eat_kind(&TokenKind::Assign) {
            Some(self.parse_expr())
        } else {
            None
        };
        Param {
            span: span_from(start, self.prev_span()),
            external_name: external,
            internal_name: internal,
            ty,
            default,
        }
    }

    fn parse_inheritance(&mut self) -> Option<InheritanceClause> {
        if !self.eat_kind(&TokenKind::Colon) {
            return None;
        }
        let start = self.prev_span().start;
        let mut items = vec![self.parse_type()];
        while self.eat_kind(&TokenKind::Comma) {
            items.push(self.parse_type());
        }
        Some(InheritanceClause {
            span: span_from(start, self.prev_span()),
            items,
        })
    }

    fn parse_where_clause(&mut self) -> Vec<GenericConstraint> {
        if !self.eat_kind(&TokenKind::Where) {
            return Vec::new();
        }
        let mut constraints = Vec::new();
        loop {
            let start = self.current_span().start;
            let subject = self.parse_type();
            if self.eat_kind(&TokenKind::EqEq) {
                let other = self.parse_type();
                constraints.push(GenericConstraint {
                    span: span_from(start, self.prev_span()),
                    subject,
                    bound: GenericBound::SameType(other),
                });
            } else if self.eat_kind(&TokenKind::Colon) {
                let bound = self.parse_type();
                constraints.push(GenericConstraint {
                    span: span_from(start, self.prev_span()),
                    subject,
                    bound: GenericBound::Type(bound),
                });
            }
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        constraints
    }

    fn parse_return_type_and_throws(&mut self) -> (Option<Type>, ThrowsClause) {
        if self.check_kind(&TokenKind::Throws) {
            let throws = self.parse_throws_clause();
            let return_type = if self.eat_kind(&TokenKind::Arrow) {
                Some(self.parse_type())
            } else {
                None
            };
            return (return_type, throws);
        }
        let return_type = if self.eat_kind(&TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        let throws = self.parse_throws_clause();
        (return_type, throws)
    }

    fn parse_throws_clause(&mut self) -> ThrowsClause {
        let start = self.current_span().start;
        if !self.eat_kind(&TokenKind::Throws) {
            return ThrowsClause {
                span: Span::EMPTY,
                types: Vec::new(),
            };
        }
        let mut types = Vec::new();
        if self.eat_kind(&TokenKind::LParen) {
            while !self.check_kind(&TokenKind::RParen) {
                types.push(self.parse_type());
                if !self.eat_kind(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect_kind(&TokenKind::RParen);
        }
        ThrowsClause {
            span: span_from(start, self.prev_span()),
            types,
        }
    }

    /// `#deprecated "msg"` (and future decl directives) → attributes on the following decl.
    fn parse_hash_decl_directives(&mut self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        while self.check_kind(&TokenKind::Hash) {
            let start = self.current_span().start;
            // Peek directive name without consuming if it's a statement directive.
            let name = match self.tokens.get(self.pos + 1).map(|t| &t.kind) {
                Some(TokenKind::Ident(n)) => n.clone(),
                Some(TokenKind::Defer) => "defer".into(),
                Some(TokenKind::If) => "if".into(),
                _ => break,
            };
            if name == "defer" || name == "die" || name == "if" || name == "declare" {
                break;
            }
            self.bump(); // Hash
            self.bump(); // Ident
            let mut args = Vec::new();
            if name == "deprecated" {
                if let TokenKind::String(s) = self.current_kind().clone() {
                    let span = self.current_span();
                    self.bump();
                    args.push(Expr::LiteralString {
                        span,
                        parts: vec![StringPart::Text(s)],
                    });
                } else {
                    self.error_expected("string message after `#deprecated`", self.current_span());
                }
            }
            attrs.push(Attribute {
                span: span_from(start, self.prev_span()),
                name,
                args,
            });
        }
        attrs
    }

    fn parse_attributes(&mut self) -> Vec<Attribute> {
        let mut attrs = Vec::new();
        while self.check_kind(&TokenKind::At) {
            let start = self.current_span().start;
            self.bump();
            // Soft-keywords allowed as attribute names (`@throws`).
            let name = if self.eat_kind(&TokenKind::Throws) {
                "throws".into()
            } else {
                self.parse_ident()
            };
            let mut args = Vec::new();
            if self.eat_kind(&TokenKind::LParen) {
                while !self.check_kind(&TokenKind::RParen) {
                    // Allow labeled attribute args: `parallel: false` or `parallel = false`.
                    if self.check_ident_start()
                        && (self.peek_kind_is(1, &TokenKind::Colon)
                            || self.peek_kind_is(1, &TokenKind::Assign))
                    {
                        let _label = self.parse_ident();
                        self.bump(); // Colon or Assign
                    }
                    args.push(self.parse_expr());
                    if !self.eat_kind(&TokenKind::Comma) {
                        break;
                    }
                }
                self.eat_kind(&TokenKind::RParen);
            }
            attrs.push(Attribute {
                span: span_from(start, self.prev_span()),
                name,
                args,
            });
        }
        attrs
    }

    fn parse_access(&mut self) -> Option<Access> {
        match self.current_kind() {
            TokenKind::Open => {
                self.bump();
                Some(Access::Open)
            }
            TokenKind::Public => {
                self.bump();
                Some(Access::Public)
            }
            TokenKind::Internal => {
                self.bump();
                Some(Access::Internal)
            }
            TokenKind::Private => {
                self.bump();
                Some(Access::Private)
            }
            TokenKind::Protected => {
                self.bump();
                Some(Access::Protected)
            }
            TokenKind::Fileprivate => {
                self.bump();
                Some(Access::Fileprivate)
            }
            _ => None,
        }
    }

    /// `private(set)` / `fileprivate(set)` / `internal(set)` after a leading access modifier.
    fn parse_setter_access(&mut self) -> Option<Access> {
        let setter = match self.current_kind() {
            TokenKind::Private => Access::Private,
            TokenKind::Fileprivate => Access::Fileprivate,
            TokenKind::Internal => Access::Internal,
            _ => return None,
        };
        // Need `access ( set )` — peek without consuming the access if not `(set)`.
        if !self.peek_kind_is(1, &TokenKind::LParen) {
            return None;
        }
        self.bump(); // private/fileprivate/internal
        self.expect_kind(&TokenKind::LParen);
        if !self.eat_kind(&TokenKind::Set) {
            self.error_expected("`set`", self.current_span());
        }
        self.expect_kind(&TokenKind::RParen);
        Some(setter)
    }

    fn parse_var_modifiers(&mut self) -> ParamModifiers {
        let mut mods = ParamModifiers::default();
        if self.eat_kind(&TokenKind::Weak) {
            mods.weak = true;
        }
        if self.eat_kind(&TokenKind::Unowned) {
            mods.unowned = true;
        }
        mods
    }

    fn parse_param_modifiers(&mut self) -> ParamModifiers {
        let mut mods = ParamModifiers::default();
        if self.eat_kind(&TokenKind::Inout) {
            mods.inout = true;
        }
        if self.eat_kind(&TokenKind::Weak) {
            mods.weak = true;
        }
        if self.eat_kind(&TokenKind::Unowned) {
            mods.unowned = true;
        }
        if self.eat_kind(&TokenKind::Let) {
            mods.storage = Some(StorageKind::Let);
        } else if self.eat_kind(&TokenKind::Var) {
            mods.storage = Some(StorageKind::Var);
        }
        mods
    }

    fn parse_qualified_name(&mut self) -> QualifiedName {
        let start = self.current_span().start;
        let mut segments = vec![self.parse_ident()];
        while self.eat_kind(&TokenKind::Dot) {
            if self.check_kind(&TokenKind::Star) {
                break;
            }
            segments.push(self.parse_ident());
        }
        QualifiedName {
            span: span_from(start, self.prev_span()),
            segments,
        }
    }

    fn parse_ident(&mut self) -> String {
        match self.bump() {
            Some(Token {
                kind: TokenKind::Ident(s),
                ..
            }) => s,
            // Soft keywords usable as member / case names.
            Some(Token {
                kind: TokenKind::Get,
                ..
            }) => "get".into(),
            Some(Token {
                kind: TokenKind::Set,
                ..
            }) => "set".into(),
            Some(Token {
                kind: TokenKind::DidSet,
                ..
            }) => "didSet".into(),
            Some(Token {
                kind: TokenKind::WillSet,
                ..
            }) => "willSet".into(),
            Some(Token {
                kind: TokenKind::Init,
                ..
            }) => "init".into(),
            Some(Token {
                kind: TokenKind::SelfLower,
                ..
            }) => "self".into(),
            Some(Token {
                kind: TokenKind::SelfUpper,
                ..
            }) => "Self".into(),
            Some(Token {
                kind: TokenKind::Typealias,
                ..
            }) => "typealias".into(),
            Some(tok) => {
                self.error_expected("identifier", tok.span);
                String::new()
            }
            None => {
                self.error_eof();
                String::new()
            }
        }
    }

    // ─── Expressions (precedence climbing) ─────────────────────────────

    fn parse_expr(&mut self) -> Expr {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Expr {
        let expr = self.parse_try();
        if self.is_assign_op() {
            let op = self.parse_assign_op();
            let start = expr.span();
            let value = self.parse_assignment();
            return Expr::Assign {
                span: start.merge(value.span()),
                target: Box::new(expr),
                op,
                value: Box::new(value),
            };
        }
        expr
    }

    fn parse_try(&mut self) -> Expr {
        if self.eat_kind(&TokenKind::Try) {
            let start = self.prev_span().start;
            let optional = self.eat_kind(&TokenKind::Question);
            let force = !optional && self.eat_kind(&TokenKind::Bang);
            let inner = self.parse_try();
            return Expr::Try {
                span: span_from(start, inner.span()),
                expr: Box::new(inner),
                optional,
                force,
            };
        }
        self.parse_await()
    }

    fn parse_await(&mut self) -> Expr {
        if self.eat_kind(&TokenKind::Await) {
            let start = self.prev_span().start;
            let inner = self.parse_await();
            return Expr::Await {
                span: span_from(start, inner.span()),
                expr: Box::new(inner),
            };
        }
        self.parse_nil_coalesce()
    }

    fn parse_nil_coalesce(&mut self) -> Expr {
        let mut expr = self.parse_ternary();
        while self.eat_kind(&TokenKind::NilCoalesce) {
            let start = expr.span();
            let right = self.parse_ternary();
            expr = Expr::NilCoalesce {
                span: start.merge(right.span()),
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_ternary(&mut self) -> Expr {
        let expr = self.parse_logical_or();
        if self.eat_kind(&TokenKind::Question) {
            let start = expr.span();
            let then_expr = self.parse_expr();
            self.expect_kind(&TokenKind::Colon);
            let else_expr = self.parse_expr();
            return Expr::Ternary {
                span: start.merge(else_expr.span()),
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }
        expr
    }

    fn parse_logical_or(&mut self) -> Expr {
        let mut expr = self.parse_logical_and();
        loop {
            if self.eat_kind(&TokenKind::OrOr) || self.eat_kind(&TokenKind::Or) {
                let start = expr.span();
                let right = self.parse_logical_and();
                expr = Expr::Binary {
                    span: start.merge(right.span()),
                    op: BinaryOp::Or,
                    left: Box::new(expr),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        expr
    }

    fn parse_logical_and(&mut self) -> Expr {
        let mut expr = self.parse_equality();
        loop {
            if self.eat_kind(&TokenKind::AndAnd) || self.eat_kind(&TokenKind::And) {
                let start = expr.span();
                let right = self.parse_equality();
                expr = Expr::Binary {
                    span: start.merge(right.span()),
                    op: BinaryOp::And,
                    left: Box::new(expr),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }
        expr
    }

    fn parse_equality(&mut self) -> Expr {
        let mut expr = self.parse_relational();
        loop {
            let op = if self.eat_kind(&TokenKind::EqEq) {
                BinaryOp::Eq
            } else if self.eat_kind(&TokenKind::NotEq) {
                BinaryOp::NotEq
            } else {
                break;
            };
            let start = expr.span();
            let right = self.parse_relational();
            expr = Expr::Binary {
                span: start.merge(right.span()),
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_relational(&mut self) -> Expr {
        let mut expr = self.parse_range();
        // `expr is Type` / `expr is Type name`
        if self.eat_kind(&TokenKind::Is) {
            let start = expr.span();
            let ty = self.parse_type_atom();
            let binding = if self.check_ident_start() {
                Some(self.parse_ident())
            } else {
                None
            };
            let end = binding
                .as_ref()
                .map(|_| self.prev_span())
                .unwrap_or_else(|| ty.span());
            return Expr::Is {
                span: start.merge(end),
                expr: Box::new(expr),
                ty,
                binding,
            };
        }
        loop {
            let op = if self.eat_kind(&TokenKind::Lt) {
                BinaryOp::Lt
            } else if self.eat_kind(&TokenKind::Le) {
                BinaryOp::Le
            } else if self.eat_kind(&TokenKind::Gt) {
                BinaryOp::Gt
            } else if self.eat_kind(&TokenKind::Ge) {
                BinaryOp::Ge
            } else {
                break;
            };
            let start = expr.span();
            let right = self.parse_range();
            expr = Expr::Binary {
                span: start.merge(right.span()),
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_range(&mut self) -> Expr {
        let expr = self.parse_additive();
        if self.eat_kind(&TokenKind::DotDotDot) {
            let start = expr.span();
            let end = self.parse_additive();
            return Expr::Range {
                span: start.merge(end.span()),
                start: Box::new(expr),
                end: Box::new(end),
                inclusive: true,
            };
        }
        if self.eat_kind(&TokenKind::DotDotLt) {
            let start = expr.span();
            let end = self.parse_additive();
            return Expr::Range {
                span: start.merge(end.span()),
                start: Box::new(expr),
                end: Box::new(end),
                inclusive: false,
            };
        }
        expr
    }

    fn parse_additive(&mut self) -> Expr {
        let mut expr = self.parse_multiplicative();
        loop {
            let op = if self.eat_kind(&TokenKind::Plus) {
                BinaryOp::Add
            } else if self.eat_kind(&TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            let start = expr.span();
            let right = self.parse_multiplicative();
            expr = Expr::Binary {
                span: start.merge(right.span()),
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut expr = self.parse_cast();
        loop {
            let op = if self.eat_kind(&TokenKind::Star) {
                BinaryOp::Mul
            } else if self.eat_kind(&TokenKind::Slash) {
                BinaryOp::Div
            } else if self.eat_kind(&TokenKind::Percent) {
                BinaryOp::Mod
            } else {
                break;
            };
            let start = expr.span();
            let right = self.parse_cast();
            expr = Expr::Binary {
                span: start.merge(right.span()),
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        expr
    }

    fn parse_cast(&mut self) -> Expr {
        let mut expr = self.parse_prefix();
        loop {
            if self.eat_kind(&TokenKind::As) {
                let kind = if self.eat_kind(&TokenKind::Question) {
                    CastKind::AsOptional
                } else if self.eat_kind(&TokenKind::Bang) {
                    CastKind::AsForced
                } else {
                    CastKind::As
                };
                let start = expr.span();
                let ty = self.parse_type_atom();
                expr = Expr::Cast {
                    span: start.merge(ty.span()),
                    expr: Box::new(expr),
                    ty,
                    kind,
                };
            } else {
                break;
            }
        }
        expr
    }

    fn parse_prefix(&mut self) -> Expr {
        let start = self.current_span().start;
        if self.eat_kind(&TokenKind::Bang) || self.eat_kind(&TokenKind::Not) {
            let operand = self.parse_prefix();
            return Expr::Unary {
                span: span_from(start, operand.span()),
                op: UnaryOp::Not,
                operand: Box::new(operand),
            };
        }
        if self.eat_kind(&TokenKind::Minus) {
            let operand = self.parse_prefix();
            return Expr::Unary {
                span: span_from(start, operand.span()),
                op: UnaryOp::Neg,
                operand: Box::new(operand),
            };
        }
        if self.eat_kind(&TokenKind::Plus) {
            let operand = self.parse_prefix();
            return Expr::Unary {
                span: span_from(start, operand.span()),
                op: UnaryOp::Pos,
                operand: Box::new(operand),
            };
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary_expr();
        loop {
            if self.eat_kind(&TokenKind::Question) && self.eat_kind(&TokenKind::Assign) {
                // try? handled in parse_try
            }
            if self.eat_kind(&TokenKind::LParen) {
                let args = self.parse_call_args();
                self.expect_kind(&TokenKind::RParen);
                let start = expr.span();
                expr = Expr::Call {
                    span: start.merge(self.prev_span()),
                    callee: Box::new(expr),
                    args,
                    try_kind: TryCallKind::None,
                };
                continue;
            }
            if self.eat_kind(&TokenKind::QuestionDot) {
                if self.eat_kind(&TokenKind::LParen) {
                    let args = self.parse_call_args();
                    self.expect_kind(&TokenKind::RParen);
                    let start = expr.span();
                    expr = Expr::Call {
                        span: start.merge(self.prev_span()),
                        callee: Box::new(expr),
                        args,
                        try_kind: TryCallKind::None,
                    };
                } else if self.eat_kind(&TokenKind::LBracket) {
                    let indices = self.parse_expr_list();
                    self.expect_kind(&TokenKind::RBracket);
                    let start = expr.span();
                    expr = Expr::Index {
                        span: start.merge(self.prev_span()),
                        base: Box::new(expr),
                        indices,
                        optional: true,
                    };
                } else {
                    let name = self.parse_ident();
                    let start = expr.span();
                    expr = Expr::Member {
                        span: start.merge(self.prev_span()),
                        base: Box::new(expr),
                        name,
                        optional: true,
                    };
                }
                continue;
            }
            if self.eat_kind(&TokenKind::Dot) {
                let name = self.parse_ident();
                let start = expr.span();
                expr = Expr::Member {
                    span: start.merge(self.prev_span()),
                    base: Box::new(expr),
                    name,
                    optional: false,
                };
                continue;
            }
            if self.eat_kind(&TokenKind::LBracket) {
                let indices = self.parse_expr_list();
                self.expect_kind(&TokenKind::RBracket);
                let start = expr.span();
                expr = Expr::Index {
                    span: start.merge(self.prev_span()),
                    base: Box::new(expr),
                    indices,
                    optional: false,
                };
                continue;
            }
            if self.eat_kind(&TokenKind::Bang) {
                let start = expr.span();
                expr = Expr::ForceUnwrap {
                    span: start.merge(self.prev_span()),
                    expr: Box::new(expr),
                };
                continue;
            }
            if self.check_kind(&TokenKind::Catch) {
                let start = expr.span();
                let catches = vec![self.parse_catch_clause()];
                expr = Expr::Catch {
                    span: start.merge(self.prev_span()),
                    expr: Box::new(expr),
                    catches,
                };
                continue;
            }
            break;
        }
        expr
    }

    fn parse_primary_expr(&mut self) -> Expr {
        let start = self.current_span().start;
        if self.check_kind(&TokenKind::If) {
            return self.parse_if_expr();
        }
        if self.check_kind(&TokenKind::Switch) {
            return self.parse_switch_expr();
        }
        if self.check_kind(&TokenKind::Do) {
            self.bump();
            let body = self.parse_block();
            let mut catches = Vec::new();
            while self.check_kind(&TokenKind::Catch) {
                catches.push(self.parse_catch_clause());
            }
            return Expr::DoCatch {
                span: span_from(start, self.prev_span()),
                body,
                catches,
            };
        }
        if self.check_kind(&TokenKind::Unsafe) {
            self.bump();
            let body = if self.check_kind(&TokenKind::LBrace) {
                UnsafeBody::Block(self.parse_block())
            } else {
                UnsafeBody::Expr(Box::new(self.parse_expr()))
            };
            return Expr::Unsafe {
                span: span_from(start, self.prev_span()),
                body,
            };
        }
        if self.eat_kind(&TokenKind::LBrace) {
            // closure
            self.pos -= 1;
            return self.parse_closure_or_block_expr();
        }
        // `(a, b) => expr` — peek before consuming `(`, because `parse_lambda` eats it.
        if self.check_kind(&TokenKind::LParen) && self.check_lambda() {
            return self.parse_lambda(start);
        }
        if self.eat_kind(&TokenKind::LParen) {
            let expr = self.parse_expr();
            self.expect_kind(&TokenKind::RParen);
            return Expr::Paren {
                span: span_from(start, self.prev_span()),
                expr: Box::new(expr),
            };
        }
        if let Some(lit) = self.parse_literal() {
            return lit;
        }
        if let TokenKind::DollarIdent(name) = self.current_kind().clone() {
            let span = self.current_span();
            self.bump();
            return Expr::CompileConst { span, name };
        }
        // Implicit enum member: `.empty`, `.some(v)`
        if self.eat_kind(&TokenKind::Dot) {
            let name = self.parse_ident();
            let span = span_from(start, self.prev_span());
            let mut expr = Expr::Ident {
                span,
                name: format!(".{name}"),
            };
            if self.eat_kind(&TokenKind::LParen) {
                let args = self.parse_call_args();
                self.expect_kind(&TokenKind::RParen);
                expr = Expr::Call {
                    span: span.merge(self.prev_span()),
                    callee: Box::new(expr),
                    args,
                    try_kind: TryCallKind::None,
                };
            }
            return expr;
        }
        if self.eat_kind(&TokenKind::SelfLower) {
            return Expr::SelfExpr {
                span: span_from(start, self.prev_span()),
                kind: SelfKind::Instance,
            };
        }
        if self.eat_kind(&TokenKind::SelfUpper) {
            return Expr::SelfExpr {
                span: span_from(start, self.prev_span()),
                kind: SelfKind::Type,
            };
        }
        if self.eat_kind(&TokenKind::Super) {
            return Expr::SelfExpr {
                span: span_from(start, self.prev_span()),
                kind: SelfKind::Super,
            };
        }
        if self.check_ident_start() {
            let name = self.parse_ident();
            // Disambiguate `Name<T>(...)` generics from `a < b` comparisons.
            let generics = if self.looks_like_generic_args_ahead() {
                self.parse_type_generic_args()
            } else {
                Vec::new()
            };
            if !generics.is_empty() {
                // Generic specialization call: Name<T>(...)
                let span = span_from(start, self.prev_span());
                let ident = Expr::Ident { span, name: name.clone() };
                if self.eat_kind(&TokenKind::LParen) {
                    let args = self.parse_call_args();
                    self.expect_kind(&TokenKind::RParen);
                    return Expr::Call {
                        span: span.merge(self.prev_span()),
                        callee: Box::new(ident),
                        args,
                        try_kind: TryCallKind::None,
                    };
                }
                return ident;
            }
            return Expr::Ident {
                span: span_from(start, self.prev_span()),
                name,
            };
        }
        self.error_expected("expression", self.current_span());
        self.bump();
        Expr::Error {
            span: self.current_span(),
        }
    }

    fn parse_if_expr(&mut self) -> Expr {
        let start = self.current_span().start;
        let (arms, else_branch) = self.parse_if_expr_parts();
        let end = else_branch
            .as_ref()
            .map(|e| match e {
                IfElse::Block(b) => b.span,
                IfElse::Expr(e) => e.span(),
                IfElse::If(e) => e.span(),
            })
            .or_else(|| arms.last().map(|a| a.span))
            .unwrap_or_else(|| self.current_span());
        Expr::If {
            span: span_from(start, end),
            arms,
            else_branch,
        }
    }

    fn parse_if_expr_parts(&mut self) -> (Vec<IfArm>, Option<IfElse>) {
        let mut arms = Vec::new();
        // One `if` arm; further arms only via `else if` (not a following bare `if`).
        if self.eat_kind(&TokenKind::If) {
            let start = self.prev_span().start;
            let condition = self.parse_if_condition();
            let body = if self.eat_kind(&TokenKind::FatArrow) {
                IfBody::Arrow(Box::new(self.parse_expr()))
            } else {
                IfBody::Block(self.parse_block())
            };
            arms.push(IfArm {
                span: span_from(start, self.prev_span()),
                condition,
                body,
            });
        }
        let else_branch = if self.eat_kind(&TokenKind::Else) {
            if self.check_kind(&TokenKind::If) {
                Some(IfElse::If(Box::new(self.parse_if_expr())))
            } else if self.eat_kind(&TokenKind::FatArrow) {
                Some(IfElse::Expr(Box::new(self.parse_expr())))
            } else {
                Some(IfElse::Block(self.parse_block()))
            }
        } else {
            None
        };
        (arms, else_branch)
    }

    fn parse_if_condition(&mut self) -> IfCondition {
        if self.check_kind(&TokenKind::Let) {
            self.bump();
            let pat = self.parse_pattern();
            let init = if self.eat_kind(&TokenKind::Assign) {
                Some(self.parse_expr())
            } else {
                None
            };
            IfCondition::Let(pat, init)
        } else if self.check_kind(&TokenKind::Var) {
            self.bump();
            let pat = self.parse_pattern();
            let init = if self.eat_kind(&TokenKind::Assign) {
                Some(self.parse_expr())
            } else {
                None
            };
            IfCondition::Var(pat, init)
        } else {
            IfCondition::Expr(self.parse_expr())
        }
    }

    fn parse_switch_expr(&mut self) -> Expr {
        let start = self.current_span().start;
        self.expect_kind(&TokenKind::Switch);
        let subject = Box::new(self.parse_expr());
        self.expect_kind(&TokenKind::LBrace);
        let mut arms = Vec::new();
        let mut default = None;
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if self.eat_kind(&TokenKind::Default) {
                self.expect_kind(&TokenKind::FatArrow);
                default = Some(Box::new(self.parse_expr()));
                continue;
            }
            let arm_start = self.current_span().start;
            let pattern = self.parse_pattern();
            let guard = if self.eat_kind(&TokenKind::Where) {
                Some(self.parse_expr())
            } else {
                None
            };
            let body = if self.eat_kind(&TokenKind::FatArrow) {
                SwitchBody::Arrow(Box::new(self.parse_expr()))
            } else {
                SwitchBody::Block(self.parse_block())
            };
            arms.push(SwitchArm {
                span: span_from(arm_start, self.prev_span()),
                pattern,
                guard,
                body,
            });
            self.eat_kind(&TokenKind::Comma);
        }
        self.eat_kind(&TokenKind::RBrace);
        Expr::Switch {
            span: span_from(start, self.prev_span()),
            subject,
            arms,
            default,
        }
    }

    fn parse_lambda(&mut self, start: u32) -> Expr {
        self.expect_kind(&TokenKind::LParen);
        let mut params = Vec::new();
        while !self.check_kind(&TokenKind::RParen) {
            let pstart = self.current_span().start;
            let name = if self.check_ident_start() {
                Some(self.parse_ident())
            } else {
                None
            };
            let ty = if self.eat_kind(&TokenKind::Colon) {
                Some(self.parse_type())
            } else {
                None
            };
            params.push(ClosureParam {
                span: span_from(pstart, self.prev_span()),
                name,
                ty,
                pattern: None,
            });
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect_kind(&TokenKind::RParen);
        self.expect_kind(&TokenKind::FatArrow);
        let body = self.parse_expr();
        Expr::Lambda {
            span: span_from(start, body.span()),
            params,
            body: Box::new(body),
        }
    }

    fn parse_closure_or_block_expr(&mut self) -> Expr {
        let start = self.current_span().start;
        if self.check_kind(&TokenKind::LBrace) {
            let block = self.parse_block();
            // Treat as inline block expression via unsafe hack — use DoCatch empty? 
            // Represent as closure with empty params
            return Expr::Closure {
                span: block.span,
                params: Vec::new(),
                return_type: None,
                body: ClosureBody::Block(block),
            };
        }
        Expr::Error { span: Span::new(start, start) }
    }

    fn parse_literal(&mut self) -> Option<Expr> {
        let start = self.current_span().start;
        let tok = self.bump()?;
        match tok.kind {
            TokenKind::Integer(v) => Some(Expr::LiteralInt {
                span: span_from(start, tok.span),
                value: v,
            }),
            TokenKind::Float(v) => Some(Expr::LiteralFloat {
                span: span_from(start, tok.span),
                value: v,
            }),
            TokenKind::String(s) => Some(Expr::LiteralString {
                span: span_from(start, tok.span),
                parts: parse_string_parts(&s),
            }),
            TokenKind::Bool(b) => Some(Expr::LiteralBool {
                span: span_from(start, tok.span),
                value: b,
            }),
            TokenKind::Nil => Some(Expr::LiteralNil {
                span: span_from(start, tok.span),
            }),
            TokenKind::Character(c) => Some(Expr::LiteralChar {
                span: span_from(start, tok.span),
                value: c,
            }),
            TokenKind::LBracket => {
                self.pos -= 1;
                let es = self.parse_array_or_dict(start);
                Some(es)
            }
            _ => {
                self.pos -= 1;
                None
            }
        }
    }

    fn parse_array_or_dict(&mut self, start: u32) -> Expr {
        self.expect_kind(&TokenKind::LBracket);
        if self.check_kind(&TokenKind::RBracket) {
            self.bump();
            return Expr::Array {
                span: span_from(start, self.prev_span()),
                elements: Vec::new(),
            };
        }
        let first = self.parse_expr();
        if self.eat_kind(&TokenKind::Colon) {
            let mut entries = vec![(first, self.parse_expr())];
            while self.eat_kind(&TokenKind::Comma) {
                if self.check_kind(&TokenKind::RBracket) {
                    break;
                }
                let k = self.parse_expr();
                self.expect_kind(&TokenKind::Colon);
                entries.push((k, self.parse_expr()));
            }
            self.expect_kind(&TokenKind::RBracket);
            Expr::Dictionary {
                span: span_from(start, self.prev_span()),
                entries,
            }
        } else {
            let mut elements = vec![first];
            while self.eat_kind(&TokenKind::Comma) {
                if self.check_kind(&TokenKind::RBracket) {
                    break;
                }
                elements.push(self.parse_expr());
            }
            self.expect_kind(&TokenKind::RBracket);
            Expr::Array {
                span: span_from(start, self.prev_span()),
                elements,
            }
        }
    }

    fn parse_call_args(&mut self) -> Vec<CallArg> {
        let mut args = Vec::new();
        while !self.check_kind(&TokenKind::RParen) && !self.check_kind(&TokenKind::Eof) {
            let start = self.current_span().start;
            let label = if self.check_ident_start() {
                let id = self.parse_ident();
                if self.eat_kind(&TokenKind::Colon) {
                    Some(id)
                } else {
                    self.pos -= 1;
                    None
                }
            } else {
                None
            };
            let value = self.parse_expr();
            args.push(CallArg {
                span: span_from(start, value.span()),
                label,
                value,
            });
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        args
    }

    fn parse_expr_list(&mut self) -> Vec<Expr> {
        let mut exprs = Vec::new();
        while !self.check_kind(&TokenKind::RBracket) {
            exprs.push(self.parse_expr());
            if !self.eat_kind(&TokenKind::Comma) {
                break;
            }
        }
        exprs
    }

    // ─── Helpers ───────────────────────────────────────────────────────

    fn synchronize_top_level(&mut self) {
        self.bump();
        while !self.check_kind(&TokenKind::Eof) {
            if self.current_kind().is_top_level_start() {
                return;
            }
            self.bump();
        }
    }

    fn synchronize_member(&mut self) {
        self.bump();
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if self.is_member_start() {
                return;
            }
            self.bump();
        }
    }

    fn synchronize_stmt(&mut self) {
        self.bump();
        while !self.check_kind(&TokenKind::RBrace) && !self.check_kind(&TokenKind::Eof) {
            if self.check_kind(&TokenKind::Semicolon) {
                self.bump();
                return;
            }
            if matches!(
                self.current_kind(),
                TokenKind::Let
                    | TokenKind::Var
                    | TokenKind::If
                    | TokenKind::While
                    | TokenKind::For
                    | TokenKind::Return
                    | TokenKind::LBrace
            ) {
                return;
            }
            self.bump();
        }
    }

    fn is_top_level_decl_start(&self) -> bool {
        if self.check_kind(&TokenKind::At) || self.check_kind(&TokenKind::Hash) {
            return true;
        }
        matches!(
            self.current_kind(),
            TokenKind::Namespace
                | TokenKind::Import
                | TokenKind::Extension
                | TokenKind::Func
                | TokenKind::Let
                | TokenKind::Var
                | TokenKind::Const
                | TokenKind::Struct
                | TokenKind::Class
                | TokenKind::Actor
                | TokenKind::Enum
                | TokenKind::Protocol
                | TokenKind::Async
                | TokenKind::Public
                | TokenKind::Internal
                | TokenKind::Private
                | TokenKind::Protected
                | TokenKind::Fileprivate
                | TokenKind::Unsafe
        )
    }

    fn is_member_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Func
                | TokenKind::Var
                | TokenKind::Let
                | TokenKind::Init
                | TokenKind::Deinit
                | TokenKind::Async
                | TokenKind::Public
                | TokenKind::Private
                | TokenKind::Protected
                | TokenKind::Internal
                | TokenKind::Virtual
                | TokenKind::Override
                | TokenKind::Abstract
                | TokenKind::Sealed
                | TokenKind::At
                | TokenKind::Struct
                | TokenKind::Class
                | TokenKind::Actor
        )
    }

    fn is_expr_start(&self) -> bool {
        self.is_literal_start()
            || self.check_ident_start()
            || matches!(
                self.current_kind(),
                TokenKind::LParen
                    | TokenKind::LBrace
                    | TokenKind::If
                    | TokenKind::Switch
                    | TokenKind::Try
                    | TokenKind::Await
                    | TokenKind::SelfLower
                    | TokenKind::SelfUpper
                    | TokenKind::Super
                    | TokenKind::Bang
                    | TokenKind::Not
                    | TokenKind::Minus
                    | TokenKind::Do
                    | TokenKind::Unsafe
                    | TokenKind::DollarIdent(_)
            )
    }

    #[allow(dead_code)]
    fn is_type_arg_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::At
                | TokenKind::SelfUpper
                | TokenKind::SelfLower
        ) || self.check_ident_start()
    }

    fn is_literal_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Integer(_)
                | TokenKind::Float(_)
                | TokenKind::String(_)
                | TokenKind::Bool(_)
                | TokenKind::Nil
                | TokenKind::Character(_)
                | TokenKind::LBracket
        )
    }

    fn is_pattern_start(&self) -> bool {
        self.check_ident_start()
            || self.check_kind(&TokenKind::LParen)
            || self.check_kind(&TokenKind::Dot)
            || self.is_literal_start()
    }

    fn check_lambda(&self) -> bool {
        let mut i = self.pos + 1;
        while i < self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::RParen => {
                    return self
                        .tokens
                        .get(i + 1)
                        .map(|t| matches!(t.kind, TokenKind::FatArrow))
                        .unwrap_or(false);
                }
                TokenKind::Eof => return false,
                _ => i += 1,
            }
        }
        false
    }

    fn is_assign_op(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Assign
                | TokenKind::PlusEq
                | TokenKind::MinusEq
                | TokenKind::StarEq
                | TokenKind::SlashEq
                | TokenKind::PercentEq
        )
    }

    fn parse_assign_op(&mut self) -> AssignOp {
        if self.eat_kind(&TokenKind::PlusEq) {
            AssignOp::Add
        } else if self.eat_kind(&TokenKind::MinusEq) {
            AssignOp::Sub
        } else if self.eat_kind(&TokenKind::StarEq) {
            AssignOp::Mul
        } else if self.eat_kind(&TokenKind::SlashEq) {
            AssignOp::Div
        } else if self.eat_kind(&TokenKind::PercentEq) {
            AssignOp::Mod
        } else {
            self.bump();
            AssignOp::Assign
        }
    }

    fn check_ident_start(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Ident(_))
    }

    /// True when `Ident` is followed by `<...>` that looks like type args (not `a < b`).
    /// Requires a closing `>` whose following token is `(` or `.`.
    fn looks_like_generic_args_ahead(&self) -> bool {
        if !self.check_kind(&TokenKind::Lt) {
            return false;
        }
        let Some(first) = self.tokens.get(self.pos + 1) else {
            return false;
        };
        match &first.kind {
            TokenKind::Integer(_)
            | TokenKind::Float(_)
            | TokenKind::String(_)
            | TokenKind::Bool(_)
            | TokenKind::Nil
            | TokenKind::Minus
            | TokenKind::LParen => return false,
            TokenKind::Ident(_) | TokenKind::SelfUpper | TokenKind::LBracket => {}
            _ => return false,
        }
        let mut i = self.pos + 1;
        let mut depth = 1i32;
        while i < self.tokens.len() && depth > 0 {
            match &self.tokens[i].kind {
                TokenKind::Lt => depth += 1,
                TokenKind::Gt => depth -= 1,
                TokenKind::Ge | TokenKind::Le => return false,
                TokenKind::EqEq
                | TokenKind::NotEq
                | TokenKind::AndAnd
                | TokenKind::OrOr
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Assign
                | TokenKind::Eof => return false,
                _ => {}
            }
            i += 1;
        }
        if depth != 0 {
            return false;
        }
        // `i` is one past the closing `>`.
        matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::LParen) | Some(TokenKind::Dot)
        )
    }

    fn peek_kind_is(&self, offset: usize, kind: &TokenKind) -> bool {
        let Some(tok) = self.tokens.get(self.pos + offset) else {
            return matches!(kind, TokenKind::Eof);
        };
        std::mem::discriminant(&tok.kind) == std::mem::discriminant(kind)
    }

    fn check_kind(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current_kind()) == std::mem::discriminant(kind)
            || matches!(
                (&self.current_kind(), kind),
                (TokenKind::Ident(_), TokenKind::Ident(_))
                    | (TokenKind::Integer(_), TokenKind::Integer(_))
                    | (TokenKind::Float(_), TokenKind::Float(_))
                    | (TokenKind::String(_), TokenKind::String(_))
                    | (TokenKind::Bool(_), TokenKind::Bool(_))
            )
    }

    fn eat_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check_kind(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_kind(&mut self, kind: &TokenKind) -> bool {
        if self.eat_kind(kind) {
            true
        } else {
            self.error_expected(format!("{kind:?}"), self.current_span());
            false
        }
    }

    fn current_kind(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn current_text(&self) -> String {
        match self.current_kind() {
            TokenKind::Ident(s) => s,
            _ => String::new(),
        }
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or(Span::new(self.source_len, self.source_len))
    }

    fn prev_span(&self) -> Span {
        self.tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span)
            .unwrap_or(Span::EMPTY)
    }

    fn bump(&mut self) -> Option<Token> {
        if self.check_kind(&TokenKind::Eof) {
            return None;
        }
        let tok = self.tokens.get(self.pos)?.clone();
        self.pos += 1;
        Some(tok)
    }

    fn error_expected(&mut self, expected: impl Into<String>, span: Span) {
        let got = format!("{:?}", self.current_kind());
        self.diagnostic(
            codes::UNEXPECTED_TOKEN,
            format!("unexpected token `{got}`; expected {}", expected.into()),
            span,
        );
    }

    fn error_eof(&mut self) {
        self.diagnostic(codes::UNEXPECTED_EOF, "unexpected end of file", self.current_span());
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostic(code, message, span);
    }

    fn diagnostic(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic {
            code: code.to_string(),
            message: message.into(),
            span,
            severity: Severity::Error,
            file: None,
        });
    }
}

trait SpanExt {
    fn span(&self) -> Span;
}

impl SpanExt for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::LiteralInt { span, .. }
            | Expr::LiteralFloat { span, .. }
            | Expr::LiteralString { span, .. }
            | Expr::LiteralBool { span, .. }
            | Expr::LiteralNil { span }
            | Expr::LiteralChar { span, .. }
            | Expr::Ident { span, .. }
            | Expr::CompileConst { span, .. }
            | Expr::SelfExpr { span, .. }
            | Expr::Array { span, .. }
            | Expr::Dictionary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. }
            | Expr::Member { span, .. }
            | Expr::Index { span, .. }
            | Expr::Assign { span, .. }
            | Expr::If { span, .. }
            | Expr::Switch { span, .. }
            | Expr::Closure { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Try { span, .. }
            | Expr::Await { span, .. }
            | Expr::Cast { span, .. }
            | Expr::Is { span, .. }
            | Expr::Ternary { span, .. }
            | Expr::NilCoalesce { span, .. }
            | Expr::ForceUnwrap { span, .. }
            | Expr::Range { span, .. }
            | Expr::DoCatch { span, .. }
            | Expr::Catch { span, .. }
            | Expr::Unsafe { span, .. }
            | Expr::Paren { span, .. }
            | Expr::Error { span } => *span,
        }
    }
}

impl SpanExt for Type {
    fn span(&self) -> Span {
        match self {
            Type::Named { span, .. }
            | Type::Optional { span, .. }
            | Type::Tuple { span, .. }
            | Type::Function { span, .. }
            | Type::Array { span, .. }
            | Type::Dictionary { span, .. }
            | Type::Attributed { span, .. }
            | Type::Error { span } => *span,
        }
    }
}

impl SpanExt for IfArm {
    fn span(&self) -> Span {
        self.span
    }
}

fn span_from(start: u32, end: Span) -> Span {
    Span::new(start, end.end)
}

fn parse_string_parts(raw: &str) -> Vec<StringPart> {
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if chars.peek() == Some(&'(') {
                if !text.is_empty() {
                    parts.push(StringPart::Text(std::mem::take(&mut text)));
                }
                chars.next();
                let mut depth = 1i32;
                let mut inner = String::new();
                for ch in chars.by_ref() {
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    inner.push(ch);
                }
                let expr = parse_interpolation_expr(&inner);
                parts.push(StringPart::Interpolation(Box::new(expr)));
            } else if let Some(next) = chars.next() {
                text.push(match next {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    other => other,
                });
            }
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() || parts.is_empty() {
        parts.push(StringPart::Text(text));
    }
    parts
}

fn parse_interpolation_expr(inner: &str) -> Expr {
    let (tokens, lex_diags) = lex(inner);
    let source_len = inner.len() as u32;
    let mut parser = Parser {
        tokens,
        pos: 0,
        diagnostics: lex_diags,
        source_len,
    };
    parser.parse_expr()
}
