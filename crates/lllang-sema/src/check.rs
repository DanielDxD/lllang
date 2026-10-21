//! Name resolution and type checking (Phase 2).

use crate::hir::{
    self, BinaryOp, Body, Callee, Expr, Function, InterpPart, Module, Pattern, ProtoMethod,
    Stmt, SwitchArm, TryKind, TypeDef, UnaryOp,
};
use crate::index::{SymbolIndex, SymbolKind};
use crate::prelude::{
    self, builtin_funcs, builtin_members, builtin_type_methods, is_known_import,
    prelude_error_types,
};
use crate::types::Ty;
use indexmap::IndexMap;
use lllang_syntax::{
    parse, CallArg, CatchBody, CatchClause, Decl, Diagnostic, Expr as AExpr, FuncBody, FuncDecl,
    GuardCondition, IfArm, IfBody, IfCondition, IfElse, InheritanceClause, MemberDecl,
    Pattern as APat, PatternBinding, SelfKind, Severity, Span, Stmt as AStmt, StringPart,
    SwitchArm as ASwitchArm, SwitchBody, ThrowsClause, TryCallKind, Type as ATy,
    BinaryOp as ABinaryOp, UnaryOp as AUnaryOp,
};

mod diag {
    pub const LL1000: &str = "LL1000";
    pub const LL1001: &str = "LL1001";
    pub const LL1003: &str = "LL1003";
    pub const LL2000: &str = "LL2000";
    pub const LL2002: &str = "LL2002";
    pub const LL2005: &str = "LL2005";
    pub const LL2008: &str = "LL2008";
    pub const LL_MAIN: &str = "LL1007";
}

#[derive(Clone)]
struct FuncSig {
    params: Vec<(String, Ty)>,
    ret: Ty,
    throws: Option<Ty>,
}

#[derive(Clone)]
struct EnumCaseInfo {
    fields: Vec<(String, Ty)>,
}

#[derive(Clone)]
enum TypeKind {
    Struct,
    Class,
    Enum,
    Protocol,
}

#[derive(Clone)]
struct TypeEntry {
    kind: TypeKind,
    fields: IndexMap<String, Ty>,
    methods: IndexMap<String, FuncSig>,
    conforms: Vec<String>,
    parent: Option<String>,
    enum_cases: IndexMap<String, EnumCaseInfo>,
    protocol_methods: IndexMap<String, (Vec<(String, Ty)>, Ty, Option<FuncBody>)>,
    virtual_methods: Vec<String>,
}

struct PendingFunc {
    decl: FuncDecl,
    owner: Option<String>,
    self_ty: Option<Ty>,
    /// Namespace active when the function was declared (for multi-file packages).
    namespace: String,
    file: String,
}

pub fn analyze_sources(sources: &[(String, String)]) -> crate::SemaResult {
    let mut checker = Checker::new();
    let mut all_decls: Vec<(String, Decl)> = Vec::new();

    for (path, src) in sources {
        let result = parse(src);
        for mut d in result.diagnostics {
            if d.file.is_none() {
                d.file = Some(path.clone());
            }
            checker.diagnostics.push(d);
        }
        if checker
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error && d.file.as_deref() == Some(path.as_str()))
        {
            continue;
        }
        for d in result.file.decls {
            all_decls.push((path.clone(), d));
        }
    }

    let decls_only: Vec<Decl> = all_decls.iter().map(|(_, d)| d.clone()).collect();
    checker.collect_decls(&decls_only);
    checker.register_types_with_files(&all_decls);
    checker.collect_functions_with_files(&all_decls);
    checker.check_all();
    checker.finish_module()
}

struct Checker {
    diagnostics: Vec<Diagnostic>,
    namespace: String,
    module_name: String,
    types: IndexMap<String, TypeEntry>,
    /// Simple name → (signature, fully-mangled name including declaring namespace).
    module_funcs: IndexMap<String, (FuncSig, String)>,
    pending: Vec<PendingFunc>,
    unknown_imports: Vec<String>,
    hir_types: Vec<TypeDef>,
    hir_funcs: Vec<Function>,
    checked_proto_defaults: IndexMap<String, IndexMap<String, Body>>,
    index: SymbolIndex,
    current_file: String,
}

impl Checker {
    fn new() -> Self {
        let mut types = IndexMap::new();
        for p in prelude::prelude_protocols() {
            types.insert(
                p.to_string(),
                TypeEntry {
                    kind: TypeKind::Protocol,
                    fields: IndexMap::new(),
                    methods: IndexMap::new(),
                    conforms: vec![],
                    parent: None,
                    enum_cases: IndexMap::new(),
                    protocol_methods: IndexMap::new(),
                    virtual_methods: vec![],
                },
            );
        }
        for e in prelude_error_types() {
            types.insert(
                e.to_string(),
                TypeEntry {
                    kind: TypeKind::Enum,
                    fields: IndexMap::new(),
                    methods: IndexMap::new(),
                    conforms: vec!["Error".into()],
                    parent: None,
                    enum_cases: IndexMap::new(),
                    protocol_methods: IndexMap::new(),
                    virtual_methods: vec![],
                },
            );
        }
        // Namespace / stdlib stub types.
        for n in [
            "JSON",
            "TOON",
            "Http",
            "ChatModel",
            "ToolRegistry",
            "LlmContext",
            "OllamaProvider",
            "Health",
        ] {
            types.insert(
                n.to_string(),
                TypeEntry {
                    kind: TypeKind::Struct,
                    fields: IndexMap::new(),
                    methods: IndexMap::new(),
                    conforms: vec![],
                    parent: None,
                    enum_cases: IndexMap::new(),
                    protocol_methods: IndexMap::new(),
                    virtual_methods: vec![],
                },
            );
        }
        if let Some(tr) = types.get_mut("ToolRegistry") {
            tr.methods.insert(
                "register".into(),
                FuncSig {
                    params: vec![("tool".into(), Ty::Named("Any".into()))],
                    ret: Ty::Void,
                    throws: None,
                },
            );
        }
        if let Some(op) = types.get_mut("OllamaProvider") {
            op.fields.insert("model".into(), Ty::String);
        }
        Self {
            diagnostics: Vec::new(),
            namespace: String::new(),
            module_name: "Module".into(),
            types,
            module_funcs: IndexMap::new(),
            pending: Vec::new(),
            unknown_imports: Vec::new(),
            hir_types: Vec::new(),
            hir_funcs: Vec::new(),
            checked_proto_defaults: IndexMap::new(),
            index: SymbolIndex::default(),
            current_file: String::new(),
        }
    }

    fn finish_module(self) -> crate::SemaResult {
        crate::SemaResult {
            module: Module {
                name: self.module_name,
                functions: self.hir_funcs,
                types: self.hir_types,
            },
            diagnostics: self.diagnostics,
            index: self.index,
        }
    }

    fn collect_decls(&mut self, decls: &[Decl]) {
        let mut unit_namespaces = Vec::new();
        for decl in decls {
            match decl {
                Decl::Namespace(n) => {
                    let path = n.path.segments.join(".");
                    unit_namespaces.push(path.clone());
                    self.namespace = path.clone();
                    self.module_name = path;
                }
                Decl::Import(i) => {
                    let path = i.path.segments.join(".");
                    if !is_known_import(&path) {
                        self.unknown_imports.push(path);
                    }
                }
                _ => {}
            }
        }
        // Path-deps / multi-file units: imports of namespaces defined in this unit are fine.
        self.unknown_imports
            .retain(|imp| !unit_namespaces.iter().any(|ns| ns == imp || ns.starts_with(&format!("{imp}."))));
    }

    fn register_types_with_files(&mut self, decls: &[(String, Decl)]) {
        for (file, decl) in decls {
            self.current_file = file.clone();
            match decl {
                Decl::Struct(s) => self.register_struct(s),
                Decl::Class(c) => self.register_class(c),
                Decl::Enum(e) => self.register_enum(e),
                Decl::Protocol(p) => self.register_protocol(p),
                _ => {}
            }
        }
    }

    fn conforms_from_inheritance(inheritance: &Option<InheritanceClause>) -> Vec<String> {
        inheritance
            .as_ref()
            .map(|i| {
                i.items
                    .iter()
                    .filter_map(|t| type_name_from_ast(t))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn register_struct(&mut self, s: &lllang_syntax::StructDecl) {
        let name = s.name.clone();
        let mut fields = IndexMap::new();
        for p in &s.primary_ctor {
            fields.insert(p.name.clone(), self.resolve_ast_type(&p.ty));
        }
        for m in &s.members {
            if let MemberDecl::Var(v) = m {
                for b in &v.bindings {
                    if let APat::Ident { name, .. } = &b.pattern {
                        let ty = b
                            .ty
                            .as_ref()
                            .map(|t| self.resolve_ast_type(t))
                            .unwrap_or(Ty::Error);
                        fields.insert(name.clone(), ty);
                    }
                }
            }
        }
        let conforms = Self::conforms_from_inheritance(&s.inheritance);
        self.types.insert(
            name.clone(),
            TypeEntry {
                kind: TypeKind::Struct,
                fields,
                methods: IndexMap::new(),
                conforms,
                parent: None,
                enum_cases: IndexMap::new(),
                protocol_methods: IndexMap::new(),
                virtual_methods: vec![],
            },
        );
        self.index.define_global(
            name.clone(),
            &self.current_file,
            s.span,
            Some(&Ty::Struct(name)),
            SymbolKind::Type,
        );
    }

    fn register_class(&mut self, c: &lllang_syntax::ClassDecl) {
        let name = c.name.clone();
        let mut fields = IndexMap::new();
        for p in &c.primary_ctor {
            fields.insert(p.name.clone(), self.resolve_ast_type(&p.ty));
        }
        for m in &c.members {
            if let MemberDecl::Var(v) = m {
                for b in &v.bindings {
                    if let APat::Ident { name, .. } = &b.pattern {
                        let ty = b
                            .ty
                            .as_ref()
                            .map(|t| self.resolve_ast_type(t))
                            .unwrap_or(Ty::Error);
                        fields.insert(name.clone(), ty);
                    }
                }
            }
        }
        let mut conforms = Self::conforms_from_inheritance(&c.inheritance);
        let parent = c.inheritance.as_ref().and_then(|i| {
            i.items.first().and_then(|t| {
                let n = type_name_from_ast(t)?;
                if self.types.get(&n).map(|e| matches!(e.kind, TypeKind::Class)).unwrap_or(false) {
                    Some(n)
                } else {
                    None
                }
            })
        });
        if let Some(ref p) = parent {
            conforms.retain(|x| x != p);
        }
        self.types.insert(
            name.clone(),
            TypeEntry {
                kind: TypeKind::Class,
                fields,
                methods: IndexMap::new(),
                conforms,
                parent,
                enum_cases: IndexMap::new(),
                protocol_methods: IndexMap::new(),
                virtual_methods: vec![],
            },
        );
        self.index.define_global(
            name.clone(),
            &self.current_file,
            c.span,
            Some(&Ty::Class(name)),
            SymbolKind::Type,
        );
    }

    fn register_enum(&mut self, e: &lllang_syntax::EnumDecl) {
        let name = e.name.clone();
        let mut enum_cases = IndexMap::new();
        for c in &e.cases {
            let fields: Vec<_> = c
                .payload
                .iter()
                .map(|(n, t)| (n.clone(), self.resolve_ast_type(t)))
                .collect();
            enum_cases.insert(c.name.clone(), EnumCaseInfo { fields });
        }
        let conforms = Self::conforms_from_inheritance(&e.inheritance);
        self.types.insert(
            name.clone(),
            TypeEntry {
                kind: TypeKind::Enum,
                fields: IndexMap::new(),
                methods: IndexMap::new(),
                conforms,
                parent: None,
                enum_cases,
                protocol_methods: IndexMap::new(),
                virtual_methods: vec![],
            },
        );
        self.index.define_global(
            name.clone(),
            &self.current_file,
            e.span,
            Some(&Ty::Enum(name)),
            SymbolKind::Type,
        );
    }

    fn register_protocol(&mut self, p: &lllang_syntax::ProtocolDecl) {
        let name = p.name.clone();
        let mut protocol_methods = IndexMap::new();
        let mut methods = IndexMap::new();
        for m in &p.members {
            if let MemberDecl::Func(f) = m {
                let (params, _self_ty) = self.func_params(f, Some(Ty::Protocol(name.clone())));
                let ret = f
                    .return_type
                    .as_ref()
                    .map(|t| self.resolve_ast_type(t))
                    .unwrap_or(Ty::Void);
                let throws = throws_type(&f.throws);
                protocol_methods.insert(
                    f.name.clone(),
                    (params.clone(), ret.clone(), body_from_decl(f)),
                );
                methods.insert(
                    f.name.clone(),
                    FuncSig {
                        params,
                        ret,
                        throws,
                    },
                );
            }
        }
        let conforms = Self::conforms_from_inheritance(&p.inheritance);
        self.types.insert(
            name.clone(),
            TypeEntry {
                kind: TypeKind::Protocol,
                fields: IndexMap::new(),
                methods,
                conforms,
                parent: None,
                enum_cases: IndexMap::new(),
                protocol_methods,
                virtual_methods: vec![],
            },
        );
        self.index.define_global(
            name.clone(),
            &self.current_file,
            p.span,
            Some(&Ty::Protocol(name)),
            SymbolKind::Type,
        );
    }

    fn collect_functions_with_files(&mut self, decls: &[(String, Decl)]) {
        let mut ns = String::new();
        for (file, decl) in decls {
            self.current_file = file.clone();
            match decl {
                Decl::Namespace(n) => {
                    ns = n.path.segments.join(".");
                    self.namespace = ns.clone();
                    self.module_name = ns.clone();
                }
                Decl::Func(f) => {
                    self.index.define_global(
                        &f.name,
                        file,
                        f.span,
                        None,
                        SymbolKind::Function,
                    );
                    self.pending.push(PendingFunc {
                        decl: f.clone(),
                        owner: None,
                        self_ty: None,
                        namespace: ns.clone(),
                        file: file.clone(),
                    });
                }
                Decl::Struct(s) => {
                    self.collect_type_funcs(
                        &s.name,
                        Ty::Struct(s.name.clone()),
                        &s.members,
                        &ns,
                        file,
                    )
                }
                Decl::Class(c) => {
                    self.collect_type_funcs(
                        &c.name,
                        Ty::Class(c.name.clone()),
                        &c.members,
                        &ns,
                        file,
                    )
                }
                Decl::Enum(e) => {
                    self.collect_type_funcs(
                        &e.name,
                        Ty::Enum(e.name.clone()),
                        &e.members,
                        &ns,
                        file,
                    )
                }
                Decl::Protocol(_) => {}
                _ => {}
            }
        }
    }

    fn collect_type_funcs(
        &mut self,
        owner: &str,
        self_ty: Ty,
        members: &[MemberDecl],
        namespace: &str,
        file: &str,
    ) {
        for m in members {
            if let MemberDecl::Func(f) = m {
                self.index.define_global(
                    &f.name,
                    file,
                    f.span,
                    None,
                    SymbolKind::Function,
                );
                self.pending.push(PendingFunc {
                    decl: f.clone(),
                    owner: Some(owner.to_string()),
                    self_ty: Some(self_ty.clone()),
                    namespace: namespace.to_string(),
                    file: file.to_string(),
                });
            }
        }
    }

    fn check_all(&mut self) {
        let pending: Vec<_> = self.pending.drain(..).collect();
        // Check type methods first so protocol defaults can be synthesized before free funcs
        // (e.g. `main`) call them.
        let (methods, free_funcs): (Vec<_>, Vec<_>) =
            pending.into_iter().partition(|pf| pf.owner.is_some());
        // Pre-register free function signatures so later files can reference earlier ones
        // regardless of source path sort order (e.g. `main.ll` before `tools.ll`).
        for pf in &free_funcs {
            let (params, _) = self.func_params(&pf.decl, pf.self_ty.clone());
            let ret = pf
                .decl
                .return_type
                .as_ref()
                .map(|t| self.resolve_ast_type(t))
                .unwrap_or(Ty::Void);
            let throws = throws_type(&pf.decl.throws);
            let mangled = if !pf.namespace.is_empty() {
                format!("{}.{}", pf.namespace, pf.decl.name)
            } else {
                pf.decl.name.clone()
            };
            self.module_funcs.insert(
                pf.decl.name.clone(),
                (
                    FuncSig {
                        params,
                        ret,
                        throws,
                    },
                    mangled,
                ),
            );
        }
        for pf in methods {
            self.check_pending_func(pf);
        }

        self.check_protocol_defaults();
        self.synthesize_protocol_default_methods();

        for pf in free_funcs {
            self.check_pending_func(pf);
        }

        if !self.hir_funcs.iter().any(|f| f.name.ends_with(".main") || f.name == "main") {
            self.warn(
                diag::LL_MAIN,
                "executable entry point `main` not found",
                Span::EMPTY,
            );
        }

        self.build_hir_types();
    }

    /// For each concrete type conforming to a protocol, emit a method wrapper for
    /// protocol default implementations the type does not override (e.g. `Circle.label`).
    fn synthesize_protocol_default_methods(&mut self) {
        let concretes: Vec<_> = self
            .types
            .iter()
            .filter(|(_, e)| matches!(e.kind, TypeKind::Struct | TypeKind::Class))
            .map(|(n, e)| {
                let mut conforms = e.conforms.clone();
                // Inherit protocol conformances from parent classes.
                let mut parent = e.parent.clone();
                while let Some(p) = parent {
                    if let Some(pe) = self.types.get(&p) {
                        for c in &pe.conforms {
                            if !conforms.contains(c) {
                                conforms.push(c.clone());
                            }
                        }
                        parent = pe.parent.clone();
                    } else {
                        break;
                    }
                }
                (
                    n.clone(),
                    conforms,
                    e.methods.keys().cloned().collect::<Vec<_>>(),
                )
            })
            .collect();
        for (type_name, conforms, existing) in concretes {
            for proto in &conforms {
                let Some(defaults) = self.checked_proto_defaults.get(proto) else {
                    continue;
                };
                let defaults: Vec<_> = defaults
                    .iter()
                    .map(|(m, b)| (m.clone(), b.clone()))
                    .collect();
                for (method_name, body) in defaults {
                    if existing.iter().any(|m| m == &method_name) {
                        continue;
                    }
                    let mangled = format!("{type_name}.{method_name}");
                    if self.hir_funcs.iter().any(|f| f.name == mangled) {
                        continue;
                    }
                    let self_ty = if self
                        .types
                        .get(&type_name)
                        .map(|e| matches!(e.kind, TypeKind::Class))
                        .unwrap_or(false)
                    {
                        Ty::Class(type_name.clone())
                    } else {
                        Ty::Struct(type_name.clone())
                    };
                    // Register so lookup_method finds it.
                    if let Some(entry) = self.types.get_mut(&type_name) {
                        let ret = body
                            .stmts
                            .last()
                            .and_then(|s| match s {
                                Stmt::Return(Some(e)) => Some(type_of_expr(e)),
                                Stmt::Expr(e) => Some(type_of_expr(e)),
                                _ => None,
                            })
                            .unwrap_or(Ty::String);
                        entry.methods.insert(
                            method_name.clone(),
                            FuncSig {
                                params: vec![],
                                ret: ret.clone(),
                                throws: None,
                            },
                        );
                        self.hir_funcs.push(Function {
                            name: mangled,
                            params: vec![],
                            ret,
                            throws: None,
                            is_method: true,
                            self_ty: Some(self_ty),
                            is_virtual: false,
                            is_override: false,
                            body: Some(body),
                        });
                    }
                }
            }
        }
    }

    fn check_protocol_defaults(&mut self) {
        let protos: Vec<_> = self
            .types
            .iter()
            .filter(|(_, e)| matches!(e.kind, TypeKind::Protocol))
            .map(|(n, e)| (n.clone(), e.protocol_methods.clone()))
            .collect();
        for (proto_name, methods) in protos {
            for (method_name, (params, ret, body)) in methods {
                let Some(body) = body else { continue };
                if matches!(body, FuncBody::Missing) {
                    continue;
                }
                let mut scope = Scope::new();
                scope.insert("self".into(), Ty::Protocol(proto_name.clone()));
                for (n, t) in &params {
                    scope.insert(n.clone(), t.clone());
                }
                for fname in self.fields_for_self(&Some(Ty::Protocol(proto_name.clone()))) {
                    scope.insert(fname.0, fname.1);
                }
                let fdecl = FuncDecl {
                    span: Span::EMPTY,
                    attrs: vec![],
                    access: None,
                    is_async: false,
                    is_override: false,
                    is_virtual: false,
                    is_abstract: false,
                    name: method_name.clone(),
                    generics: vec![],
                    params: vec![],
                    return_type: Some(ast_type_from_ty(&ret)),
                    where_clause: vec![],
                    throws: ThrowsClause {
                        span: Span::EMPTY,
                        types: vec![],
                    },
                    body,
                };
                let body_hir = self.check_func_body(
                    &fdecl,
                    &scope,
                    Some(Ty::Protocol(proto_name.clone())),
                    None,
                );
                self.checked_proto_defaults
                    .entry(proto_name.clone())
                    .or_default()
                    .insert(method_name, body_hir);
            }
        }
    }

    fn check_pending_func(&mut self, pf: PendingFunc) {
        let f = &pf.decl;
        self.current_file = pf.file.clone();
        let (params, _self_ty) = self.func_params(f, pf.self_ty.clone());
        let ret = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_ast_type(t))
            .unwrap_or(Ty::Void);
        let throws = throws_type(&f.throws);

        let sig = FuncSig {
            params: params.clone(),
            ret: ret.clone(),
            throws: throws.clone(),
        };

        let mangled = if let Some(ref owner) = pf.owner {
            format!("{owner}.{}", f.name)
        } else if !pf.namespace.is_empty() {
            format!("{}.{}", pf.namespace, f.name)
        } else {
            f.name.clone()
        };
        // Restore declaring namespace for nested lookups during body check.
        self.namespace = pf.namespace.clone();

        if pf.owner.is_none() {
            self.module_funcs.insert(f.name.clone(), (sig.clone(), mangled.clone()));
        } else if let Some(owner) = &pf.owner {
            if let Some(entry) = self.types.get_mut(owner) {
                entry.methods.insert(f.name.clone(), sig.clone());
                if f.is_virtual {
                    entry.virtual_methods.push(f.name.clone());
                }
            }
        }

        let mut scope = Scope::new();
        if pf.self_ty.is_some() {
            scope.insert("self".into(), pf.self_ty.clone().unwrap_or(Ty::Error));
            for (fname, fty) in self.fields_for_self(&pf.self_ty) {
                scope.insert(fname, fty);
            }
        }
        for (n, t) in &params {
            // Approximate param span from func decl (full param span unavailable here).
            let id = self.index.define(
                n,
                &self.current_file,
                f.span,
                Some(t),
                SymbolKind::Param,
            );
            let _ = id;
            scope.insert_param(n.clone(), t.clone());
        }

        let body = match &f.body {
            FuncBody::Missing => None,
            _ => Some(self.check_func_body(f, &scope, pf.self_ty.clone(), throws.clone())),
        };

        self.hir_funcs.push(Function {
            name: mangled,
            params,
            ret,
            throws,
            is_method: pf.self_ty.is_some(),
            self_ty: pf.self_ty,
            is_virtual: f.is_virtual,
            is_override: f.is_override,
            body,
        });
    }

    fn fields_for_self(&self, self_ty: &Option<Ty>) -> Vec<(String, Ty)> {
        let Some(ty) = self_ty else {
            return vec![];
        };
        let name = match ty {
            Ty::Struct(n) | Ty::Class(n) => n,
            _ => return vec![],
        };
        self.types
            .get(name)
            .map(|e| e.fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    fn build_hir_types(&mut self) {
        for (name, entry) in &self.types {
            if matches!(name.as_str(), "Error" | "Equatable") && entry.protocol_methods.is_empty() && entry.fields.is_empty() && entry.enum_cases.is_empty() {
                if matches!(entry.kind, TypeKind::Protocol) {
                    self.hir_types.push(TypeDef::Protocol {
                        name: name.clone(),
                        methods: vec![],
                    });
                }
                continue;
            }
            match entry.kind {
                TypeKind::Struct => {
                    let methods: Vec<_> = self
                        .hir_funcs
                        .iter()
                        .filter(|f| f.name.starts_with(&format!("{name}.")))
                        .cloned()
                        .collect();
                    self.hir_types.push(TypeDef::Struct {
                        name: name.clone(),
                        fields: entry.fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                        methods,
                        conforms: entry.conforms.clone(),
                    });
                }
                TypeKind::Class => {
                    let methods: Vec<_> = self
                        .hir_funcs
                        .iter()
                        .filter(|f| f.name.starts_with(&format!("{name}.")))
                        .cloned()
                        .collect();
                    self.hir_types.push(TypeDef::Class {
                        name: name.clone(),
                        fields: entry.fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                        methods,
                        parent: entry.parent.clone(),
                        conforms: entry.conforms.clone(),
                        virtual_methods: entry.virtual_methods.clone(),
                    });
                }
                TypeKind::Enum => {
                    let cases: Vec<_> = entry
                        .enum_cases
                        .iter()
                        .map(|(n, c)| hir::EnumCase {
                            name: n.clone(),
                            fields: c.fields.clone(),
                        })
                        .collect();
                    self.hir_types.push(TypeDef::Enum {
                        name: name.clone(),
                        cases,
                        conforms: entry.conforms.clone(),
                    });
                }
                TypeKind::Protocol => {
                    let mut methods = vec![];
                    for (mn, (params, ret, body)) in &entry.protocol_methods {
                        let default_body = self
                            .checked_proto_defaults
                            .get(name)
                            .and_then(|m| m.get(mn))
                            .cloned()
                            .or_else(|| {
                                body.as_ref().and_then(|b| match b {
                                    FuncBody::Missing => None,
                                    _ => Some(Body { stmts: vec![] }),
                                })
                            });
                        methods.push(ProtoMethod {
                            name: mn.clone(),
                            params: params.clone(),
                            ret: ret.clone(),
                            default_body,
                        });
                    }
                    self.hir_types.push(TypeDef::Protocol {
                        name: name.clone(),
                        methods,
                    });
                }
            }
        }
    }

    fn func_params(&mut self, f: &FuncDecl, self_ty: Option<Ty>) -> (Vec<(String, Ty)>, Option<Ty>) {
        let mut params = Vec::new();
        for p in &f.params {
            if p.internal_name == "self" || p.internal_name.is_empty() && p.external_name.as_deref() == Some("self") {
                continue;
            }
            let name = p.internal_name.clone();
            params.push((name, self.resolve_ast_type(&p.ty)));
        }
        (params, self_ty)
    }

    fn check_func_body(
        &mut self,
        f: &FuncDecl,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> Body {
        let ret = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_ast_type(t))
            .unwrap_or(Ty::Void);
        match &f.body {
            FuncBody::Block(b) => {
                let mut stmts =
                    self.check_block_stmts(&b.stmts, scope, self_ty.clone(), throws.clone());
                // Specialize trailing JSON/TOON decode to the function return type.
                if let Some(Stmt::Expr(Expr::Call {
                    callee: Callee::Builtin(bid),
                    ty,
                    ..
                })) = stmts.last_mut()
                {
                    if (bid == "json_decode" || bid == "toon_decode")
                        && (ty.is_error()
                            || matches!(ty, Ty::Named(n) if n == "Decodable"))
                    {
                        *ty = ret;
                    }
                }
                Body { stmts }
            }
            FuncBody::Expr(e) => {
                let mut expr = self.check_expr(e, scope, self_ty, throws.clone(), false);
                if let Expr::Call {
                    callee: Callee::Builtin(ref bid),
                    ty,
                    ..
                } = &mut expr
                {
                    if (bid == "json_decode" || bid == "toon_decode")
                        && (ty.is_error()
                            || matches!(ty, Ty::Named(n) if n == "Decodable"))
                    {
                        *ty = ret;
                    }
                }
                Body {
                    stmts: vec![Stmt::Return(Some(expr))],
                }
            }
            FuncBody::Missing => Body { stmts: vec![] },
        }
    }

    fn check_block_stmts(
        &mut self,
        stmts: &[AStmt],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> Vec<Stmt> {
        let mut out = Vec::new();
        let mut cur = scope.clone();
        for stmt in stmts {
            out.extend(self.check_stmt(stmt, &mut cur, self_ty.clone(), throws.clone()));
        }
        out
    }

    fn check_stmt(
        &mut self,
        stmt: &AStmt,
        scope: &mut Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> Vec<Stmt> {
        match stmt {
            AStmt::Let(l) => {
                let mut result = vec![];
                for b in &l.bindings {
                    let (name, init_expr, ann_ty) = self.binding_info(b, scope, self_ty.clone(), throws.clone());
                    let had_ann = ann_ty.is_some();
                    let init = init_expr.unwrap_or(Expr::Nil);
                    let ty = ann_ty.unwrap_or_else(|| {
                        let inferred = self.expr_ty(&init, scope);
                        if inferred.is_error() {
                            type_of_expr(&init)
                        } else {
                            inferred
                        }
                    });
                    if !ty.is_error() && !ty.assignable_from(&self.expr_ty(&init, scope)) {
                        self.error_type_mismatch(&ty, &self.expr_ty(&init, scope), b.span);
                    }
                    self.index.define(
                        &name,
                        &self.current_file,
                        b.span,
                        Some(&ty),
                        SymbolKind::Local,
                    );
                    if !had_ann && !ty.is_error() {
                        self.index
                            .inlay(&self.current_file, b.span, format!(": {}", ty.display()));
                    }
                    scope.insert(name.clone(), ty.clone());
                    result.push(Stmt::Let {
                        name,
                        ty,
                        init,
                    });
                }
                result
            }
            AStmt::Var(v) => {
                let mut result = vec![];
                for b in &v.bindings {
                    let (name, init_expr, ann_ty) = self.binding_info(b, scope, self_ty.clone(), throws.clone());
                    let had_ann = ann_ty.is_some();
                    let ty = ann_ty.unwrap_or_else(|| {
                        init_expr
                            .as_ref()
                            .map(|e| type_of_expr(e))
                            .unwrap_or(Ty::Error)
                    });
                    if let Some(ref init) = init_expr {
                        if !ty.is_error() && !ty.assignable_from(&self.expr_ty(init, scope)) {
                            self.error_type_mismatch(&ty, &self.expr_ty(init, scope), b.span);
                        }
                    }
                    self.index.define(
                        &name,
                        &self.current_file,
                        b.span,
                        Some(&ty),
                        SymbolKind::Local,
                    );
                    if !had_ann && !ty.is_error() {
                        self.index
                            .inlay(&self.current_file, b.span, format!(": {}", ty.display()));
                    }
                    scope.insert(name.clone(), ty.clone());
                    result.push(Stmt::Var {
                        name,
                        ty,
                        init: init_expr,
                    });
                }
                result
            }
            AStmt::Return { value, span: _ } => {
                vec![Stmt::Return(value.as_ref().map(|e| {
                    self.check_expr(e, scope, self_ty.clone(), throws.clone(), false)
                }))]
            }
            AStmt::Throw { value, .. } => {
                // `throw .case` — Ident named ".case" from the parser's leading-dot sugar.
                if let AExpr::Ident { name, .. } = value {
                    if let Some(case) = name.strip_prefix('.') {
                        if let Some(Ty::Enum(en)) = &throws {
                            return vec![Stmt::Throw(Expr::EnumCase {
                                enum_name: en.clone(),
                                case: case.to_string(),
                                args: vec![],
                                ty: throws.clone().unwrap(),
                            })];
                        }
                    }
                }
                vec![Stmt::Throw(self.check_expr(
                    value,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    true,
                ))]
            }
            AStmt::Guard { condition, else_body, .. } => {
                let (cond_expr, extra_scope, opt_src) = self.check_guard_condition(
                    condition,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                );
                let else_b = Body {
                    stmts: self.check_block_stmts(&else_body.stmts, scope, self_ty.clone(), throws.clone()),
                };
                let mut out = vec![Stmt::Guard {
                    cond: cond_expr,
                    else_body: else_b,
                }];
                // Materialize `guard let x = opt` bindings after the guard succeeds.
                if let Some(src) = opt_src {
                    for (k, v) in extra_scope {
                        scope.insert(k.clone(), v.clone());
                        out.push(Stmt::Let {
                            name: k,
                            ty: v.clone(),
                            init: Expr::Call {
                                callee: Callee::Builtin("unwrap_optional".into()),
                                args: vec![src.clone()],
                                ty: v,
                                is_try: TryKind::None,
                            },
                        });
                    }
                } else {
                    for (k, v) in extra_scope {
                        scope.insert(k, v);
                    }
                }
                out
            }
            AStmt::If { arms, else_branch, .. } => {
                self.check_if_stmt(arms, else_branch.as_ref(), scope, self_ty, throws)
            }
            AStmt::While { condition, body, .. } => {
                let cond = self.check_expr(condition, scope, self_ty.clone(), throws.clone(), false);
                let body = Body {
                    stmts: self.check_block_stmts(&body.stmts, scope, self_ty, throws),
                };
                vec![Stmt::While { cond, body }]
            }
            AStmt::Expr { expr, .. } => {
                vec![Stmt::Expr(self.check_expr(
                    expr,
                    scope,
                    self_ty,
                    throws.clone(),
                    false,
                ))]
            }
            AStmt::Block(b) => self.check_block_stmts(&b.stmts, scope, self_ty, throws),
            _ => vec![],
        }
    }

    fn check_if_stmt(
        &mut self,
        arms: &[IfArm],
        else_branch: Option<&IfElse>,
        scope: &mut Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> Vec<Stmt> {
        if let Some(arm) = arms.first() {
            let cond = match &arm.condition {
                IfCondition::Expr(e) => self.check_expr(e, scope, self_ty.clone(), throws.clone(), false),
                IfCondition::Let(p, o) | IfCondition::Var(p, o) => {
                    let init = o.as_ref().map(|e| self.check_expr(e, scope, self_ty.clone(), throws.clone(), false));
                    if let Some(ref ie) = init {
                        if let Ty::Optional(inner) = type_of_expr(ie) {
                            if let APat::Ident { name, .. } = p {
                                scope.insert(name.clone(), (*inner).clone());
                            }
                        }
                    }
                    init.unwrap_or(Expr::Bool(false))
                }
            };
            let then_body = match &arm.body {
                IfBody::Block(b) => Body {
                    stmts: self.check_block_stmts(&b.stmts, scope, self_ty.clone(), throws.clone()),
                },
                IfBody::Arrow(e) => Body {
                    stmts: vec![Stmt::Expr(self.check_expr(
                        e,
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        false,
                    ))],
                },
            };
            let else_body = else_branch.map(|eb| match eb {
                IfElse::Block(b) => Body {
                    stmts: self.check_block_stmts(&b.stmts, scope, self_ty, throws.clone()),
                },
                IfElse::Expr(e) => Body {
                    stmts: vec![Stmt::Expr(self.check_expr(
                        e,
                        scope,
                        self_ty,
                        throws,
                        false,
                    ))],
                },
                IfElse::If(inner) => match inner.as_ref() {
                    AExpr::If {
                        arms,
                        else_branch,
                        ..
                    } => Body {
                        stmts: self.check_if_stmt(
                            arms,
                            else_branch.as_ref(),
                            scope,
                            self_ty,
                            throws,
                        ),
                    },
                    other => Body {
                        stmts: vec![Stmt::Expr(self.check_expr(
                            other,
                            scope,
                            self_ty,
                            throws,
                            false,
                        ))],
                    },
                },
            });
            return vec![Stmt::If {
                cond,
                then_body,
                else_body,
            }];
        }
        vec![]
    }

    fn check_guard_condition(
        &mut self,
        cond: &GuardCondition,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> (Expr, IndexMap<String, Ty>, Option<Expr>) {
        match cond {
            GuardCondition::Expr(e) => (
                self.check_expr(e, scope, self_ty, throws, false),
                IndexMap::new(),
                None,
            ),
            GuardCondition::Let(p, e) => {
                let expr = self.check_expr(e, scope, self_ty, throws.clone(), false);
                let mut bindings = IndexMap::new();
                let ety = type_of_expr(&expr);
                if let Ty::Optional(inner) = &ety {
                    if let APat::Ident { name, .. } = p {
                        bindings.insert(name.clone(), (**inner).clone());
                    }
                } else {
                    self.error(
                        diag::LL2002,
                        "value of optional type must be unwrapped",
                        e.span(),
                    );
                }
                // Branch on "is Some".
                let is_some = Expr::Call {
                    callee: Callee::Builtin("optional_is_some".into()),
                    args: vec![expr.clone()],
                    ty: Ty::Bool,
                    is_try: TryKind::None,
                };
                (is_some, bindings, Some(expr))
            }
        }
    }

    fn binding_info(
        &mut self,
        b: &PatternBinding,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> (String, Option<Expr>, Option<Ty>) {
        let name = match &b.pattern {
            APat::Ident { name, .. } => name.clone(),
            APat::Typed { pattern, .. } => match pattern.as_ref() {
                APat::Ident { name, .. } => name.clone(),
                _ => "_".into(),
            },
            APat::Wildcard { .. } => "_".into(),
            _ => "_".into(),
        };
        let ann = b
            .ty
            .as_ref()
            .map(|t| self.resolve_ast_type(t))
            .or_else(|| pattern_type(&b.pattern).map(|t| self.resolve_ast_type(t)));
        let init = b.initializer.as_ref().map(|e| {
            let mut expr = self.check_expr(e, scope, self_ty, throws, true);
            // Specialize JSON/TOON decode return type from annotation.
            if let Some(ref expected) = ann {
                if let Expr::Call {
                    callee: Callee::Builtin(ref bid),
                    ty,
                    ..
                } = &mut expr
                {
                    if (bid == "json_decode" || bid == "toon_decode")
                        && (ty.is_error()
                            || matches!(ty, Ty::Named(n) if n == "Decodable"))
                    {
                        *ty = expected.clone();
                    }
                }
            }
            expr
        });
        (name, init, ann)
    }

    fn check_expr(
        &mut self,
        expr: &AExpr,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        match expr {
            AExpr::LiteralInt { value, .. } => {
                Expr::Int(value.parse().unwrap_or(0))
            }
            AExpr::LiteralFloat { value, .. } => {
                Expr::Float(value.parse().unwrap_or(0.0))
            }
            AExpr::LiteralBool { value, .. } => Expr::Bool(*value),
            AExpr::LiteralChar { value, .. } => Expr::Char(*value),
            AExpr::LiteralNil { .. } => Expr::Nil,
            AExpr::LiteralString { parts, .. } => self.check_string_parts(parts, scope, self_ty.clone(), throws.clone()),
            AExpr::Ident { name, span } => {
                if let Some(case) = name.strip_prefix('.') {
                    if let Some(Ty::Enum(en)) = &throws {
                        return Expr::EnumCase {
                            enum_name: en.clone(),
                            case: case.to_string(),
                            args: vec![],
                            ty: throws.clone().unwrap(),
                        };
                    }
                }
                self.check_ident(name, *span, scope)
            }
            AExpr::SelfExpr { kind, .. } => match kind {
                SelfKind::Instance => Expr::Self_,
                SelfKind::Type | SelfKind::Super => Expr::Self_,
            },
            AExpr::Unary { op, operand, span } => {
                let inner = self.check_expr(operand, scope, self_ty.clone(), throws.clone(), throws_ok);
                let (ty, uop) = match op {
                    AUnaryOp::Not => (Ty::Bool, UnaryOp::Not),
                    AUnaryOp::Neg => (self.expr_ty(&inner, scope), UnaryOp::Neg),
                    AUnaryOp::Pos => (self.expr_ty(&inner, scope), UnaryOp::Pos),
                    _ => (Ty::Error, UnaryOp::Not),
                };
                if matches!(op, AUnaryOp::Not) && self.expr_ty(&inner, scope) != Ty::Bool {
                    self.error_type_mismatch(&Ty::Bool, &self.expr_ty(&inner, scope), *span);
                }
                Expr::Unary {
                    op: uop,
                    expr: Box::new(inner),
                    ty,
                }
            }
            AExpr::Binary { op, left, right, span } => {
                let l = self.check_expr(left, scope, self_ty.clone(), throws.clone(), throws_ok);
                let r = self.check_expr(right, scope, self_ty.clone(), throws.clone(), throws_ok);
                let lty = self.expr_ty(&l, scope);
                let rty = self.expr_ty(&r, scope);
                let ty = binary_result_ty(*op, &lty, &rty);
                if *op == ABinaryOp::Eq || *op == ABinaryOp::NotEq {
                    // ok
                } else if ty == Ty::Error {
                    self.error_type_mismatch(&Ty::Int, &lty, *span);
                }
                Expr::Binary {
                    op: map_binop(*op),
                    left: Box::new(l),
                    right: Box::new(r),
                    ty,
                }
            }
            AExpr::Call {
                callee,
                args,
                try_kind,
                span,
            } => self.check_call(callee, args, *try_kind, *span, scope, self_ty, throws, throws_ok),
            AExpr::Member { base, name, .. } => {
                let b = self.check_expr(base, scope, self_ty.clone(), throws.clone(), throws_ok);
                self.check_member(b, name, scope, self_ty)
            }
            AExpr::If { arms, else_branch, .. } => {
                self.check_if_expr(arms, else_branch.as_ref(), scope, self_ty, throws, throws_ok)
            }
            AExpr::Switch {
                subject,
                arms,
                default,
                span,
            } => self.check_switch(subject, arms, default.as_deref(), *span, scope, self_ty, throws, throws_ok),
            AExpr::Try { expr, optional, .. } => {
                let inner = self.check_expr(
                    expr,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok || *optional,
                );
                if *optional {
                    Expr::TryOptional(Box::new(inner))
                } else {
                    inner
                }
            }
            // Phase 3 MVP: await is a no-op (cooperative single-threaded).
            AExpr::Await { expr, .. } => {
                self.check_expr(expr, scope, self_ty, throws, throws_ok)
            }
            AExpr::Catch { expr, catches, .. } => {
                self.check_catch(expr, catches, scope, self_ty, throws, throws_ok)
            }
            AExpr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let c = self.check_expr(condition, scope, self_ty.clone(), throws.clone(), throws_ok);
                let t = self.check_expr(then_expr, scope, self_ty.clone(), throws.clone(), throws_ok);
                let e = self.check_expr(else_expr, scope, self_ty.clone(), throws.clone(), throws_ok);
                let ty = self.expr_ty(&t, scope);
                Expr::IfExpr {
                    cond: Box::new(c),
                    then_e: Box::new(t),
                    else_e: Box::new(e),
                    ty,
                }
            }
            AExpr::Paren { expr, .. } => self.check_expr(expr, scope, self_ty, throws, throws_ok),
            AExpr::Assign { target, value, .. } => {
                let t = self.check_expr(target, scope, self_ty.clone(), throws.clone(), throws_ok);
                let v = self.check_expr(value, scope, self_ty, throws, throws_ok);
                Expr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(t),
                    right: Box::new(v),
                    ty: Ty::Void,
                }
            }
            _ => Expr::Nil,
        }
    }

    fn expr_ty(&self, expr: &Expr, scope: &Scope) -> Ty {
        match expr {
            Expr::Local(name) | Expr::Param(name) => scope.get(name).unwrap_or(Ty::Error),
            other => type_of_expr(other),
        }
    }

    fn check_string_parts(
        &mut self,
        parts: &[StringPart],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> Expr {
        let has_interpolation = parts.iter().any(|p| matches!(p, StringPart::Interpolation(_)));
        if !has_interpolation {
            let mut lit = String::new();
            for part in parts {
                if let StringPart::Text(t) = part {
                    lit.push_str(t);
                }
            }
            return Expr::StringLit(lit);
        }

        let mut interp = Vec::new();
        let mut lit = String::new();
        for part in parts {
            match part {
                StringPart::Text(t) => {
                    lit.push_str(t);
                }
                StringPart::Interpolation(e) => {
                    if !lit.is_empty() {
                        interp.push(InterpPart::Lit(std::mem::take(&mut lit)));
                    }
                    let ce = self.check_expr(e, scope, self_ty.clone(), throws.clone(), false);
                    let ety = self.expr_ty(&ce, scope);
                    if !is_stringifiable(&ety) {
                        self.error_type_mismatch(&Ty::String, &ety, e.span());
                    }
                    interp.push(InterpPart::Expr(ce));
                }
            }
        }
        if !lit.is_empty() {
            interp.push(InterpPart::Lit(lit));
        }
        if interp.len() == 1 {
            if let InterpPart::Lit(s) = &interp[0] {
                return Expr::StringLit(s.clone());
            }
        }
        Expr::StringInterp(interp)
    }

    fn check_ident(&mut self, name: &str, span: Span, scope: &Scope) -> Expr {
        if name.starts_with('.') {
            return Expr::Local(name.to_string());
        }
        if is_type_name(name) || self.types.contains_key(name) {
            if let Some(id) = self.index.lookup_name(&self.current_file, name) {
                self.index.reference(id, &self.current_file, span);
            }
            return Expr::Local(name.to_string());
        }
        if scope.get(name).is_some() {
            if let Some(id) = self.index.lookup_name(&self.current_file, name) {
                self.index.reference(id, &self.current_file, span);
            }
            return if scope.is_param(name) {
                Expr::Param(name.to_string())
            } else {
                Expr::Local(name.to_string())
            };
        }
        if self.module_funcs.contains_key(name) || builtin_funcs().contains_key(name) {
            if let Some(id) = self.index.lookup_name(&self.current_file, name) {
                self.index.reference(id, &self.current_file, span);
            }
            return Expr::Local(name.to_string());
        }
        self.error(
            diag::LL1001,
            format!("cannot find value `{name}` in scope"),
            span,
        );
        if !self.unknown_imports.is_empty() {
            self.error(
                diag::LL1003,
                format!("import `{}` not found", self.unknown_imports[0]),
                span,
            );
        }
        Expr::Local(name.to_string())
    }

    fn check_call(
        &mut self,
        callee: &AExpr,
        args: &[CallArg],
        try_kind: TryCallKind,
        span: Span,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        if let Some(case_name) = implicit_case_name(callee) {
            let empty_args = Vec::new();
            let call_args = match callee {
                AExpr::Call { args, .. } => args,
                _ => &empty_args,
            };
            let checked_args: Vec<_> = call_args
                .iter()
                .map(|a| self.check_expr(&a.value, scope, self_ty.clone(), throws.clone(), throws_ok))
                .collect();
            if let Some(ref et) = throws {
                if let Ty::Enum(en) = et {
                    return Expr::EnumCase {
                        enum_name: en.clone(),
                        case: case_name,
                        args: checked_args,
                        ty: et.clone(),
                    };
                }
            }
            if case_name == "some" && checked_args.len() == 1 {
                return Expr::OptionalSome(Box::new(checked_args.into_iter().next().unwrap()));
            }
            if case_name == "none" {
                return Expr::OptionalNone(Ty::Optional(Box::new(Ty::Error)));
            }
        }

        if let AExpr::Ident { name, .. } = callee {
            if let Some(st) = &self_ty {
                if let Some((sig, virtual_call)) = self.lookup_method(st, name) {
                    let checked = self.check_call_args(
                        args,
                        &sig.params,
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        throws_ok,
                    );
                    self.verify_try(&sig.throws, try_kind, throws_ok, span);
                    return Expr::Call {
                        callee: if virtual_call {
                            Callee::Virtual {
                                receiver: Box::new(Expr::Self_),
                                name: name.clone(),
                            }
                        } else {
                            Callee::Method {
                                receiver: Box::new(Expr::Self_),
                                name: name.clone(),
                            }
                        },
                        args: checked,
                        ty: sig.ret.clone(),
                        is_try: map_try(try_kind),
                    };
                }
            }
            if let Some(entry) = self.types.get(name) {
                if matches!(entry.kind, TypeKind::Struct | TypeKind::Class) {
                    return self.check_construct(name, args, scope, self_ty, throws, throws_ok);
                }
            }
            if name == "print" || builtin_funcs().contains_key(name) {
                let (params, ret, throws_ty, bid) = builtin_funcs()[name].clone();
                let checked = if name == "runAgent" {
                    self.check_call_args(
                        args,
                        &[
                            ("model".into(), Ty::Named("ChatModel".into())),
                            ("tools".into(), Ty::Named("ToolRegistry".into())),
                            ("user".into(), Ty::String),
                        ],
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        throws_ok,
                    )
                } else {
                    self.check_args_against(
                        args,
                        &params,
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        throws_ok,
                    )
                };
                self.verify_try(&throws_ty, try_kind, throws_ok, span);
                return Expr::Call {
                    callee: Callee::Builtin(bid.to_string()),
                    args: checked,
                    ty: ret,
                    is_try: map_try(try_kind),
                };
            }
        }

        if let AExpr::Member { base, name: method, .. } = callee {
            if let AExpr::Ident { name: type_name, .. } = base.as_ref() {
                if is_type_name(type_name) || self.types.contains_key(type_name) {
                    if let Some((params, ret, _, bid)) =
                        builtin_type_methods().get(&(type_name.clone(), method.clone()))
                    {
                        let checked = self.check_args_against(
                            args,
                            params,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        return Expr::Call {
                            callee: Callee::Builtin(bid.to_string()),
                            args: checked,
                            ty: ret.clone(),
                            is_try: map_try(try_kind),
                        };
                    }
                }
            }
            let base_expr = self.check_expr(base, scope, self_ty.clone(), throws.clone(), throws_ok);
            let base_ty = self.expr_ty(&base_expr, scope);
            {
                let type_key = match &base_ty {
                    Ty::Double => Some("Double".to_string()),
                    Ty::Named(n) | Ty::Struct(n) | Ty::Class(n) => Some(n.clone()),
                    _ => None,
                };
                if let Some(type_key) = type_key {
                if let Some((params, ret, _, bid)) =
                    builtin_type_methods().get(&(type_key.clone(), method.clone()))
                {
                    let checked = self.check_args_against(args, params, scope, self_ty.clone(), throws.clone(), throws_ok);
                    return Expr::Call {
                        callee: Callee::Builtin(bid.to_string()),
                        args: checked,
                        ty: ret.clone(),
                        is_try: map_try(try_kind),
                    };
                }
                if let Some((ret, is_prop, bid)) = builtin_members().get(&(type_key, method.clone())) {
                    if !is_prop {
                        // Instance builtin method: receiver is first arg.
                        let mut checked = vec![base_expr.clone()];
                        checked.extend(self.check_args_against(
                            args,
                            &[],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        ));
                        return Expr::Call {
                            callee: Callee::Builtin(bid.to_string()),
                            args: checked,
                            ty: ret.clone(),
                            is_try: map_try(try_kind),
                        };
                    }
                }
                } // type_key
            }
            if let Some((sig, virtual_call)) = self.lookup_method(&base_ty, method) {
                let checked = self.check_call_args(args, &sig.params, scope, self_ty.clone(), throws.clone(), throws_ok);
                self.verify_try(&sig.throws, try_kind, throws_ok, span);
                let callee = if virtual_call {
                    Callee::Virtual {
                        receiver: Box::new(base_expr),
                        name: method.clone(),
                    }
                } else {
                    Callee::Method {
                        receiver: Box::new(base_expr),
                        name: method.clone(),
                    }
                };
                return Expr::Call {
                    callee,
                    args: checked,
                    ty: sig.ret.clone(),
                    is_try: map_try(try_kind),
                };
            }
        }

        let callee_expr = self.check_expr(callee, scope, self_ty.clone(), throws.clone(), throws_ok);
        if let Expr::Local(ref n) = callee_expr {
            if let Some((sig, mangled)) = self.module_funcs.get(n).cloned() {
                let checked = self.check_call_args(args, &sig.params, scope, self_ty.clone(), throws.clone(), throws_ok);
                self.verify_try(&sig.throws, try_kind, throws_ok, span);
                return Expr::Call {
                    callee: Callee::Func(mangled),
                    args: checked,
                    ty: sig.ret,
                    is_try: map_try(try_kind),
                };
            }
        }

        let checked: Vec<_> = args
            .iter()
            .map(|a| self.check_expr(&a.value, scope, self_ty.clone(), throws.clone(), throws_ok))
            .collect();
        Expr::Call {
            callee: Callee::Func("?".into()),
            args: checked,
            ty: Ty::Error,
            is_try: map_try(try_kind),
        }
    }

    fn check_construct(
        &mut self,
        type_name: &str,
        args: &[CallArg],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        let entry = self.types.get(type_name).cloned();
        let ty = match entry.as_ref().map(|e| &e.kind) {
            Some(TypeKind::Struct) => Ty::Struct(type_name.to_string()),
            Some(TypeKind::Class) => Ty::Class(type_name.to_string()),
            _ => Ty::Error,
        };
        let field_order: Vec<_> = entry
            .map(|e| e.fields.keys().cloned().collect())
            .unwrap_or_default();
        let mut named_args = IndexMap::new();
        let mut positional: Vec<Expr> = vec![];
        for arg in args {
            let val = self.check_expr(&arg.value, scope, self_ty.clone(), throws.clone(), throws_ok);
            if let Some(label) = &arg.label {
                named_args.insert(label.clone(), val);
            } else {
                positional.push(val);
            }
        }
        let mut out_args = vec![];
        for (i, fname) in field_order.iter().enumerate() {
            let expr = if let Some(e) = named_args.swap_remove(fname) {
                e
            } else if i < positional.len() {
                positional[i].clone()
            } else {
                Expr::Nil
            };
            out_args.push((fname.clone(), expr));
        }
        Expr::Construct {
            ty_name: type_name.to_string(),
            args: out_args,
            ty,
        }
    }

    fn check_member(
        &mut self,
        base: Expr,
        name: &str,
        scope: &Scope,
        self_ty: Option<Ty>,
    ) -> Expr {
        // `funcName.self` — function reference (LLM tool registration).
        if name == "self" {
            if let Expr::Local(ref n) = base {
                if self.module_funcs.contains_key(n) || builtin_funcs().contains_key(n) {
                    return base;
                }
            }
        }
        let base_ty = self.expr_ty(&base, scope);
        if base_ty == Ty::String {
            if let Some((ty, _, bid)) = builtin_members().get(&("String".into(), name.to_string())) {
                return Expr::Call {
                    callee: Callee::Builtin(bid.to_string()),
                    args: vec![base],
                    ty: ty.clone(),
                    is_try: TryKind::None,
                };
            }
        }
        if let Some(tn) = type_name_key(&base_ty) {
            if let Some(entry) = self.types.get(tn) {
                if let Some(fty) = entry.fields.get(name) {
                    return Expr::Member {
                        base: Box::new(base),
                        name: name.to_string(),
                        ty: fty.clone(),
                    };
                }
            }
        }
        if name == "isEmpty" && base_ty == Ty::String {
            return Expr::Member {
                base: Box::new(base),
                name: name.to_string(),
                ty: Ty::Bool,
            };
        }
        if let Some((sig, _)) = self.lookup_method(&base_ty, name) {
            return Expr::Member {
                base: Box::new(base),
                name: name.to_string(),
                ty: Ty::Func {
                    params: sig.params.iter().map(|(_, t)| t.clone()).collect(),
                    ret: Box::new(sig.ret.clone()),
                    throws: sig.throws.clone().map(Box::new),
                },
            };
        }
        let _ = scope;
        let _ = self_ty;
        Expr::Member {
            base: Box::new(base),
            name: name.to_string(),
            ty: Ty::Error,
        }
    }

    fn lookup_method(&self, receiver: &Ty, name: &str) -> Option<(FuncSig, bool)> {
        let type_name = type_name_key(receiver)?;
        if let Some(entry) = self.types.get(type_name) {
            if let Some(m) = entry.methods.get(name) {
                let virt = matches!(entry.kind, TypeKind::Class) && entry.virtual_methods.contains(&name.to_string());
                return Some((m.clone(), virt));
            }
            for proto in &entry.conforms {
                if let Some(p) = self.types.get(proto) {
                    if let Some(m) = p.methods.get(name) {
                        return Some((m.clone(), false));
                    }
                }
            }
            if let Some(parent) = &entry.parent {
                if let Some(p) = self.types.get(parent) {
                    if let Some(m) = p.methods.get(name) {
                        let virt = p.virtual_methods.contains(&name.to_string());
                        return Some((m.clone(), virt));
                    }
                }
            }
        }
        None
    }

    fn check_call_args(
        &mut self,
        args: &[CallArg],
        params: &[(String, Ty)],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Vec<Expr> {
        let mut by_label: IndexMap<String, Expr> = IndexMap::new();
        let mut positional = Vec::new();
        for arg in args {
            let v = self.check_expr(&arg.value, scope, self_ty.clone(), throws.clone(), throws_ok);
            if let Some(l) = &arg.label {
                by_label.insert(l.clone(), v);
            } else {
                positional.push(v);
            }
        }
        let mut out = Vec::new();
        let mut pi = 0;
        for (pname, pty) in params {
            let expr = if let Some(e) = by_label.swap_remove(pname) {
                e
            } else if pi < positional.len() {
                let e = positional[pi].clone();
                pi += 1;
                e
            } else {
                Expr::Nil
            };
            if !pty.is_error() && !pty.assignable_from(&type_of_expr(&expr)) {
                self.error_type_mismatch(pty, &type_of_expr(&expr), Span::EMPTY);
            }
            out.push(expr);
        }
        out
    }

    fn check_args_against(
        &mut self,
        args: &[CallArg],
        params: &[Ty],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Vec<Expr> {
        let param_pairs: Vec<_> = params
            .iter()
            .enumerate()
            .map(|(i, t)| (format!("arg{i}"), t.clone()))
            .collect();
        self.check_call_args(args, &param_pairs, scope, self_ty, throws, throws_ok)
    }

    fn verify_try(&mut self, callee_throws: &Option<Ty>, try_kind: TryCallKind, throws_ok: bool, span: Span) {
        if callee_throws.is_some() && try_kind == TryCallKind::None && !throws_ok {
            if let Some(e) = callee_throws {
                self.error(
                    diag::LL2005,
                    format!("call requires `try`; function throws `{}`", e.display()),
                    span,
                );
            }
        }
    }

    fn check_if_expr(
        &mut self,
        arms: &[IfArm],
        else_branch: Option<&IfElse>,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        if let Some(arm) = arms.first() {
            let cond = match &arm.condition {
                IfCondition::Expr(e) => self.check_expr(e, scope, self_ty.clone(), throws.clone(), throws_ok),
                _ => Expr::Bool(true),
            };
            let then_e = match &arm.body {
                IfBody::Arrow(e) => self.check_expr(e, scope, self_ty.clone(), throws.clone(), throws_ok),
                IfBody::Block(b) => b
                    .stmts
                    .last()
                    .map(|s| match s {
                        AStmt::Expr { expr, .. } => {
                            self.check_expr(expr, scope, self_ty.clone(), throws.clone(), throws_ok)
                        }
                        AStmt::Return { value: Some(v), .. } => {
                            self.check_expr(v, scope, self_ty.clone(), throws.clone(), throws_ok)
                        }
                        _ => Expr::Nil,
                    })
                    .unwrap_or(Expr::Nil),
            };
            let else_e = else_branch
                .map(|eb| match eb {
                    IfElse::Expr(e) => self.check_expr(e, scope, self_ty.clone(), throws.clone(), throws_ok),
                    _ => Expr::Nil,
                })
                .unwrap_or(Expr::Nil);
            let ty = type_of_expr(&then_e);
            return Expr::IfExpr {
                cond: Box::new(cond),
                then_e: Box::new(then_e),
                else_e: Box::new(else_e),
                ty,
            };
        }
        Expr::Nil
    }

    fn check_switch(
        &mut self,
        subject: &AExpr,
        arms: &[ASwitchArm],
        default: Option<&AExpr>,
        span: Span,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        let subj = self.check_expr(subject, scope, self_ty.clone(), throws.clone(), throws_ok);
        let subj_ty = {
            let t = self.expr_ty(&subj, scope);
            if t.is_error() {
                type_of_expr(&subj)
            } else {
                t
            }
        };
        let mut hir_arms = vec![];
        let mut has_wildcard = false;
        let mut has_some = false;
        let mut has_none = false;
        let mut result_ty = Ty::Void;
        for arm in arms {
            let (pat, binds) = self.map_pattern(&arm.pattern, &subj_ty);
            if matches!(pat, Pattern::Wildcard) {
                has_wildcard = true;
            }
            if matches!(pat, Pattern::OptionalSome(_)) {
                has_some = true;
            }
            if matches!(pat, Pattern::OptionalNone) {
                has_none = true;
            }
            let mut arm_scope = scope.clone();
            for (n, t) in binds {
                arm_scope.insert(n, t);
            }
            let guard = arm.guard.as_ref().map(|g| {
                self.check_expr(g, &arm_scope, self_ty.clone(), throws.clone(), throws_ok)
            });
            let body = match &arm.body {
                SwitchBody::Arrow(e) => self.check_expr(e, &arm_scope, self_ty.clone(), throws.clone(), throws_ok),
                SwitchBody::Block(b) => {
                    let stmts = self.check_block_stmts(&b.stmts, &arm_scope, self_ty.clone(), throws.clone());
                    stmts
                        .last()
                        .and_then(|s| match s {
                            Stmt::Expr(e) | Stmt::Return(Some(e)) => Some(e.clone()),
                            _ => None,
                        })
                        .unwrap_or(Expr::Nil)
                }
            };
            result_ty = type_of_expr(&body);
            hir_arms.push(SwitchArm {
                pattern: pat,
                guard,
                body,
            });
        }
        if default.is_some() {
            has_wildcard = true;
        }
        if matches!(subj_ty, Ty::Optional(_)) && !has_wildcard && !(has_some && has_none) {
            self.error(diag::LL2008, "switch must be exhaustive", span);
        }
        Expr::Switch {
            subject: Box::new(subj),
            arms: hir_arms,
            ty: result_ty,
        }
    }

    fn map_pattern(&self, pat: &APat, subject_ty: &Ty) -> (Pattern, IndexMap<String, Ty>) {
        let mut binds = IndexMap::new();
        match pat {
            APat::Wildcard { .. } => (Pattern::Wildcard, binds),
            APat::Ident { name, .. } => {
                binds.insert(name.clone(), subject_ty.clone());
                (Pattern::Ident(name.clone()), binds)
            }
            APat::EnumCase { case, subpatterns, .. } => {
                if case == "some" && subpatterns.len() == 1 {
                    if let APat::Ident { name, .. } = &subpatterns[0] {
                        if let Ty::Optional(inner) = subject_ty {
                            binds.insert(name.clone(), inner.as_ref().clone());
                            return (Pattern::OptionalSome(name.clone()), binds);
                        }
                    }
                }
                if case == "none" {
                    return (Pattern::OptionalNone, binds);
                }
                let bindings: Vec<_> = subpatterns
                    .iter()
                    .filter_map(|p| {
                        if let APat::Ident { name, .. } = p {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                (
                    Pattern::EnumCase {
                        case: case.clone(),
                        bindings,
                    },
                    binds,
                )
            }
            APat::Literal { expr, .. } => {
                if let AExpr::LiteralInt { value, .. } = expr.as_ref() {
                    (Pattern::LitInt(value.parse().unwrap_or(0)), binds)
                } else if let AExpr::LiteralBool { value, .. } = expr.as_ref() {
                    (Pattern::LitBool(*value), binds)
                } else {
                    (Pattern::Wildcard, binds)
                }
            }
            _ => (Pattern::Wildcard, binds),
        }
    }

    fn check_catch(
        &mut self,
        expr: &AExpr,
        catches: &[CatchClause],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        let inner = self.check_expr(expr, scope, self_ty.clone(), throws.clone(), true);
        let clause = catches.first();
        let (binder, fallback) = if let Some(c) = clause {
            let binder = c
                .pattern
                .as_ref()
                .and_then(|p| {
                    if let APat::Ident { name, .. } = p {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "e".into());
            let fb = match &c.body {
                CatchBody::Arrow(e) => self.check_expr(e, scope, self_ty.clone(), throws.clone(), throws_ok),
                CatchBody::Block(b) => {
                    let stmts = self.check_block_stmts(&b.stmts, &mut scope.clone(), self_ty.clone(), throws.clone());
                    stmts
                        .into_iter()
                        .find_map(|s| match s {
                            Stmt::Expr(e) | Stmt::Return(Some(e)) => Some(e),
                            _ => None,
                        })
                        .unwrap_or(Expr::Int(0))
                }
            };
            (binder, fb)
        } else {
            ("e".into(), Expr::Nil)
        };
        let ty = type_of_expr(&fallback);
        Expr::Catch {
            expr: Box::new(inner),
            binder,
            fallback: Box::new(fallback),
            ty,
        }
    }

    fn resolve_ast_type(&mut self, ty: &ATy) -> Ty {
        match ty {
            ATy::Named { name, generics, span } => {
                match name.as_str() {
                    "Void" => return Ty::Void,
                    "Never" => return Ty::Never,
                    "Bool" => return Ty::Bool,
                    "Int" => return Ty::Int,
                    "Int8" => return Ty::Int8,
                    "Int16" => return Ty::Int16,
                    "Int32" => return Ty::Int32,
                    "Int64" => return Ty::Int64,
                    "UInt" => return Ty::UInt,
                    "Float" => return Ty::Float,
                    "Double" => return Ty::Double,
                    "Char" => return Ty::Char,
                    "String" => return Ty::String,
                    _ => {}
                };
                if self.types.contains_key(name) {
                    let entry = &self.types[name];
                    return match entry.kind {
                        TypeKind::Struct => Ty::Struct(name.clone()),
                        TypeKind::Class => Ty::Class(name.clone()),
                        TypeKind::Enum => Ty::Enum(name.clone()),
                        TypeKind::Protocol => Ty::Protocol(name.clone()),
                    };
                }
                if prelude::prelude_protocols().contains(&name.as_str()) {
                    return Ty::Protocol(name.clone());
                }
                if !generics.is_empty() {
                    let inner = self.resolve_ast_type(&generics[0]);
                    return Ty::Optional(Box::new(inner));
                }
                self.error(
                    diag::LL1000,
                    format!("cannot find type `{name}` in scope"),
                    *span,
                );
                Ty::Named(name.clone())
            }
            ATy::Optional { inner, .. } => Ty::Optional(Box::new(self.resolve_ast_type(inner))),
            ATy::Array { element, .. } => Ty::Array(Box::new(self.resolve_ast_type(element))),
            ATy::Tuple { elements, .. } => Ty::Tuple(
                elements
                    .iter()
                    .map(|(_, t)| self.resolve_ast_type(t))
                    .collect(),
            ),
            ATy::Function {
                params,
                return_type,
                throws,
                ..
            } => Ty::Func {
                params: params.iter().map(|t| self.resolve_ast_type(t)).collect(),
                ret: Box::new(self.resolve_ast_type(return_type)),
                throws: throws_type(throws).map(Box::new),
            },
            ATy::Error { .. } => Ty::Error,
            _ => Ty::Error,
        }
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(
            Diagnostic::error(code, message, span).with_file(self.current_file.clone()),
        );
    }

    fn warn(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(
            Diagnostic::warning(code, message, span).with_file(self.current_file.clone()),
        );
    }

    fn error_type_mismatch(&mut self, expected: &Ty, found: &Ty, span: Span) {
        self.error(
            diag::LL2000,
            format!(
                "type mismatch: expected `{}`, found `{}`",
                expected.display(),
                found.display()
            ),
            span,
        );
    }
}

#[derive(Clone, Default)]
struct Scope {
    vars: IndexMap<String, Ty>,
    params: IndexMap<String, ()>,
}

impl Scope {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, name: String, ty: Ty) {
        self.vars.insert(name, ty);
    }

    fn insert_param(&mut self, name: String, ty: Ty) {
        self.params.insert(name.clone(), ());
        self.vars.insert(name, ty);
    }

    fn get(&self, name: &str) -> Option<Ty> {
        self.vars.get(name).cloned()
    }

    fn is_param(&self, name: &str) -> bool {
        self.params.contains_key(name)
    }
}

fn type_name_from_ast(ty: &ATy) -> Option<String> {
    match ty {
        ATy::Named { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn throws_type(clause: &ThrowsClause) -> Option<Ty> {
    clause.types.first().map(|t| match t {
        ATy::Named { name, .. } => Ty::Enum(name.clone()),
        other => Ty::Named(type_name_from_ast(other).unwrap_or_default()),
    })
}

fn body_from_decl(f: &FuncDecl) -> Option<FuncBody> {
    match &f.body {
        FuncBody::Missing => None,
        other => Some(other.clone()),
    }
}

fn ast_type_from_ty(ty: &Ty) -> ATy {
    ATy::Named {
        span: Span::EMPTY,
        name: ty.display(),
        generics: vec![],
    }
}

fn type_name_key(ty: &Ty) -> Option<&str> {
    match ty {
        Ty::Struct(n) | Ty::Class(n) | Ty::Enum(n) | Ty::Protocol(n) | Ty::Named(n) => Some(n.as_str()),
        _ => None,
    }
}

fn type_of_expr(e: &Expr) -> Ty {
    match e {
        Expr::Int(_) => Ty::Int,
        Expr::Float(_) => Ty::Double,
        Expr::Bool(_) => Ty::Bool,
        Expr::Char(_) => Ty::Char,
        Expr::Nil => Ty::Optional(Box::new(Ty::Error)),
        Expr::StringLit(_) | Expr::StringInterp(_) => Ty::String,
        Expr::Local(_) | Expr::Param(_) | Expr::Self_ => Ty::Error,
        Expr::Call { ty, .. } => ty.clone(),
        Expr::Member { ty, .. } => ty.clone(),
        Expr::Unary { ty, .. } => ty.clone(),
        Expr::Binary { ty, .. } => ty.clone(),
        Expr::IfExpr { ty, .. } => ty.clone(),
        Expr::Switch { ty, .. } => ty.clone(),
        Expr::OptionalSome(inner) => Ty::Optional(Box::new(type_of_expr(inner))),
        Expr::OptionalNone(t) => t.clone(),
        Expr::TryOptional(inner) => Ty::Optional(Box::new(type_of_expr(inner))),
        Expr::Catch { ty, .. } => ty.clone(),
        Expr::Construct { ty, .. } => ty.clone(),
        Expr::EnumCase { ty, .. } => ty.clone(),
        Expr::Cast { to, .. } => to.clone(),
    }
}

fn binary_result_ty(op: ABinaryOp, l: &Ty, r: &Ty) -> Ty {
    match op {
        ABinaryOp::Eq | ABinaryOp::NotEq => Ty::Bool,
        ABinaryOp::And | ABinaryOp::Or => Ty::Bool,
        ABinaryOp::Add if l == &Ty::String || r == &Ty::String => Ty::String,
        ABinaryOp::Add | ABinaryOp::Sub | ABinaryOp::Mul | ABinaryOp::Div | ABinaryOp::Mod => {
            if l == &Ty::Double || r == &Ty::Double {
                Ty::Double
            } else {
                Ty::Int
            }
        }
        _ => Ty::Bool,
    }
}

fn map_binop(op: ABinaryOp) -> BinaryOp {
    match op {
        ABinaryOp::Add => BinaryOp::Add,
        ABinaryOp::Sub => BinaryOp::Sub,
        ABinaryOp::Mul => BinaryOp::Mul,
        ABinaryOp::Div => BinaryOp::Div,
        ABinaryOp::Mod => BinaryOp::Mod,
        ABinaryOp::Eq => BinaryOp::Eq,
        ABinaryOp::NotEq => BinaryOp::NotEq,
        ABinaryOp::Lt => BinaryOp::Lt,
        ABinaryOp::Le => BinaryOp::Le,
        ABinaryOp::Gt => BinaryOp::Gt,
        ABinaryOp::Ge => BinaryOp::Ge,
        ABinaryOp::And => BinaryOp::And,
        ABinaryOp::Or => BinaryOp::Or,
        _ => BinaryOp::Add,
    }
}

fn pattern_type(pat: &APat) -> Option<&ATy> {
    match pat {
        APat::Typed { ty, .. } => Some(ty),
        _ => None,
    }
}

fn is_type_name(name: &str) -> bool {
    matches!(
        name,
        "Void" | "Never"
            | "Bool"
            | "Int"
            | "Int8"
            | "Int16"
            | "Int32"
            | "Int64"
            | "UInt"
            | "Float"
            | "Double"
            | "Char"
            | "String"
    )
}

fn implicit_case_name(callee: &AExpr) -> Option<String> {
    let name = match callee {
        AExpr::Ident { name, .. } => name.as_str(),
        AExpr::Call { callee, .. } => match callee.as_ref() {
            AExpr::Ident { name, .. } => name.as_str(),
            _ => return None,
        },
        _ => return None,
    };
    name.strip_prefix('.').map(str::to_string)
}

fn map_try(k: TryCallKind) -> TryKind {
    match k {
        TryCallKind::None => TryKind::None,
        TryCallKind::Try => TryKind::Try,
        TryCallKind::TryOptional => TryKind::TryOptional,
    }
}

fn is_stringifiable(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::String | Ty::Int | Ty::Double | Ty::Float | Ty::Bool | Ty::Char
    ) || ty.is_error()
}

trait SpanExt {
    fn span(&self) -> Span;
}

impl SpanExt for AExpr {
    fn span(&self) -> Span {
        match self {
            AExpr::LiteralInt { span, .. }
            | AExpr::LiteralFloat { span, .. }
            | AExpr::LiteralBool { span, .. }
            | AExpr::LiteralNil { span }
            | AExpr::LiteralChar { span, .. }
            | AExpr::LiteralString { span, .. }
            | AExpr::Ident { span, .. }
            | AExpr::SelfExpr { span, .. }
            | AExpr::Unary { span, .. }
            | AExpr::Binary { span, .. }
            | AExpr::Call { span, .. }
            | AExpr::Member { span, .. }
            | AExpr::If { span, .. }
            | AExpr::Switch { span, .. }
            | AExpr::Try { span, .. }
            | AExpr::Catch { span, .. }
            | AExpr::Ternary { span, .. }
            | AExpr::Paren { span, .. }
            | AExpr::Assign { span, .. } => *span,
            _ => Span::EMPTY,
        }
    }
}
