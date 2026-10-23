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
    parse, AssignOp, CallArg, CatchBody, CatchClause, ClosureBody, ClosureParam, Decl, DeferBody,
    Diagnostic, Expr as AExpr, FuncBody, FuncDecl, GenericBound, GuardCondition, IfArm, IfBody,
    IfCondition, IfElse, InheritanceClause, MemberDecl, Pattern as APat, PatternBinding, SelfKind,
    Severity, Span, Stmt as AStmt, StringPart, SwitchArm as ASwitchArm, SwitchBody, ThrowsClause,
    TryCallKind, Type as ATy, BinaryOp as ABinaryOp, UnaryOp as AUnaryOp,
};

mod diag {
    pub const LL1000: &str = "LL1000";
    pub const LL1001: &str = "LL1001";
    pub const LL1003: &str = "LL1003";
    pub const LL2000: &str = "LL2000";
    pub const LL2002: &str = "LL2002";
    pub const LL2005: &str = "LL2005";
    pub const LL2008: &str = "LL2008";
    pub const LL3001: &str = "LL3001"; // construct parsed but not yet lowered
    pub const LL4001: &str = "LL4001"; // deprecated symbol used
    pub const LL_MAIN: &str = "LL1007";
}

#[derive(Clone)]
struct FuncSig {
    params: Vec<(String, Ty)>,
    ret: Ty,
    throws: Option<Ty>,
    is_async: bool,
    /// Extension `where` bounds: subject name → required protocol names.
    where_bounds: IndexMap<String, Vec<String>>,
}

#[derive(Clone)]
struct EnumCaseInfo {
    fields: Vec<(String, Ty)>,
}

#[derive(Clone, Copy)]
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
    /// Sealed method names that subclasses may not override.
    sealed_methods: Vec<String>,
    is_abstract: bool,
    is_sealed: bool,
    /// Declared with `actor` (serial mailbox MVP).
    is_actor: bool,
    /// Field names stored as weak references (class fields only).
    weak_fields: Vec<String>,
    /// Field names stored as unowned references.
    unowned_fields: Vec<String>,
    /// Computed property name → synthesized getter method (`__get_x`).
    computed_getters: IndexMap<String, String>,
    /// Computed property name → synthesized setter method (`__set_x`).
    computed_setters: IndexMap<String, String>,
    /// Stored property → `__willSet_x` method.
    will_set_observers: IndexMap<String, String>,
    /// Stored property → `__didSet_x` method.
    did_set_observers: IndexMap<String, String>,
    /// Field name → setter access from `private(set)` etc.
    setter_access: IndexMap<String, lllang_syntax::Access>,
}


struct PendingFunc {
    decl: FuncDecl,
    owner: Option<String>,
    self_ty: Option<Ty>,
    /// Namespace active when the function was declared (for multi-file packages).
    namespace: String,
    file: String,
    /// From enclosing `extension … where …` (empty for type members).
    where_bounds: IndexMap<String, Vec<String>>,
}

/// Free generic function awaiting monomorphization at call sites.
#[derive(Clone)]
struct GenericTemplate {
    decl: FuncDecl,
    namespace: String,
    file: String,
    /// Type parameter → required protocol names (`T: P` and `where T: P`).
    constraints: IndexMap<String, Vec<String>>,
}

pub fn analyze_sources(sources: &[(String, String)]) -> crate::SemaResult {
    let mut checker = Checker::new();
    let mut all_decls: Vec<(String, Decl)> = Vec::new();

    let debug = std::env::var("LANG_RELEASE")
        .map(|v| v != "1" && !v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);
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
        let mut env = lllang_syntax::CompileEnv::system(debug);
        let (file, expand_diags) = lllang_syntax::expand_file(result.file, &mut env);
        for mut d in expand_diags {
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
        for d in file.decls {
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
    next_lambda: usize,
    lambda_funcs: Vec<Function>,
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
    /// Free generic function templates (simple name → template).
    generic_templates: IndexMap<String, GenericTemplate>,
    /// Already-emitted specialized mangled names.
    specialized_funcs: IndexMap<String, FuncSig>,
    /// Active substitution while checking a specialized generic body.
    type_param_subst: IndexMap<String, Ty>,
    /// Type parameter names in scope while resolving a generic signature.
    active_type_params: Vec<String>,
    /// `@extern(cName: "...")` free functions → C symbol name.
    extern_funcs: IndexMap<String, String>,
    /// Nested `unsafe { ... }` depth (FFI calls require ≥ 1).
    unsafe_depth: usize,
    /// Inside an `async` function body (Phase 10 await checks).
    in_async: bool,
    /// `async let` bindings in the current function: name → result type after await.
    async_lets: IndexMap<String, Ty>,
    /// Async-let names already joined via `await`.
    async_lets_joined: IndexMap<String, ()>,
    /// Mangeld / simple name → deprecation message (`#deprecated`).
    deprecated: IndexMap<String, String>,
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
                    sealed_methods: vec![],
                    is_abstract: false,
                    is_sealed: false,
                    is_actor: false,
                    weak_fields: vec![],
                    unowned_fields: vec![],
                    computed_getters: IndexMap::new(),
                    computed_setters: IndexMap::new(),
                    will_set_observers: IndexMap::new(),
                    did_set_observers: IndexMap::new(),
                    setter_access: IndexMap::new(),
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
                    sealed_methods: vec![],
                    is_abstract: false,
                    is_sealed: false,
                    is_actor: false,
                    weak_fields: vec![],
                    unowned_fields: vec![],
                        computed_getters: IndexMap::new(),
                        computed_setters: IndexMap::new(),
                        will_set_observers: IndexMap::new(),
                        did_set_observers: IndexMap::new(),
                        setter_access: IndexMap::new(),
                    },
            );
        }
        // Injected `Result` — payloads are typed via `Ty::Result` (no Int erasure at the type
        // level). Runtime enum storage still uses pointer-sized slots for ok/err payloads.
        {
            let mut enum_cases = IndexMap::new();
            enum_cases.insert(
                "ok".into(),
                EnumCaseInfo {
                    fields: vec![("value".into(), Ty::Named("Any".into()))],
                },
            );
            enum_cases.insert(
                "err".into(),
                EnumCaseInfo {
                    fields: vec![("error".into(), Ty::Named("Any".into()))],
                },
            );
            types.insert(
                "Result".into(),
                TypeEntry {
                    kind: TypeKind::Enum,
                    fields: IndexMap::new(),
                    methods: IndexMap::new(),
                    conforms: vec![],
                    parent: None,
                    enum_cases,
                    protocol_methods: IndexMap::new(),
                    virtual_methods: vec![],
                    sealed_methods: vec![],
                    is_abstract: false,
                    is_sealed: false,
                    is_actor: false,
                    weak_fields: vec![],
                    unowned_fields: vec![],
                        computed_getters: IndexMap::new(),
                        computed_setters: IndexMap::new(),
                        will_set_observers: IndexMap::new(),
                        did_set_observers: IndexMap::new(),
                        setter_access: IndexMap::new(),
                    },
            );
        }
        // Namespace / stdlib stub types.
        for n in [
            "JSON",
            "TOON",
            "Http",
            "Log",
            "CLI",
            "Crypto",
            "Mutex",
            "TcpStream",
            "Task",
            "Channel",
            "WaitGroup",
            "TaskGroup",
            "CancellationToken",
            "Regex",
            "SQLManager",
            "SQLConnection",
            "MainActor",
            "ChatModel",
            "ToolRegistry",
            "LlmContext",
            "OllamaProvider",
            "Health",
            "Environment",
            "Env",
            "Identity",
            "Args",
            "File",
            "Instant",
            "Process",
            "ProcessHandle",
            "Base64",
            "Hex",
            "SecureRandom",
            "Hash",
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
                    sealed_methods: vec![],
                    is_abstract: false,
                    is_sealed: false,
                    is_actor: false,
                    weak_fields: vec![],
                    unowned_fields: vec![],
                        computed_getters: IndexMap::new(),
                        computed_setters: IndexMap::new(),
                        will_set_observers: IndexMap::new(),
                        did_set_observers: IndexMap::new(),
                        setter_access: IndexMap::new(),
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
                    is_async: false,
                    where_bounds: IndexMap::new(),
                },
            );
        }
        if let Some(op) = types.get_mut("OllamaProvider") {
            op.fields.insert("model".into(), Ty::String);
        }
        Self {
            diagnostics: Vec::new(),
            next_lambda: 0,
            lambda_funcs: Vec::new(),
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
            generic_templates: IndexMap::new(),
            specialized_funcs: IndexMap::new(),
            type_param_subst: IndexMap::new(),
            active_type_params: Vec::new(),
            extern_funcs: IndexMap::new(),
            unsafe_depth: 0,
            in_async: false,
            async_lets: IndexMap::new(),
            async_lets_joined: IndexMap::new(),
            deprecated: IndexMap::new(),
        }
    }

    fn finish_module(mut self) -> crate::SemaResult {
        self.hir_funcs.append(&mut self.lambda_funcs);
        if !self.hir_types.iter().any(|t| matches!(t, TypeDef::Enum { name, .. } if name == "Result")) {
            self.hir_types.push(TypeDef::Enum {
                name: "Result".into(),
                cases: vec![
                    hir::EnumCase {
                        name: "ok".into(),
                        fields: vec![("value".into(), Ty::Named("Any".into()))],
                    },
                    hir::EnumCase {
                        name: "err".into(),
                        fields: vec![("error".into(), Ty::Named("Any".into()))],
                    },
                ],
                conforms: vec![],
            });
        }
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

    fn reject_protected(&mut self, access: Option<lllang_syntax::Access>, span: Span) {
        if matches!(access, Some(lllang_syntax::Access::Protected)) {
            self.error(
                diag::LL2000,
                "`protected` is not part of LLLang access control (ADR 009); use `fileprivate` or `public`",
                span,
            );
        }
    }

    fn register_struct(&mut self, s: &lllang_syntax::StructDecl) {
        self.reject_protected(s.access, s.span);
        let name = s.name.clone();
        let mut fields = IndexMap::new();
        for p in &s.primary_ctor {
            fields.insert(p.name.clone(), self.resolve_ast_type(&p.ty));
        }
        let mut setter_access = IndexMap::new();
        for m in &s.members {
            if let MemberDecl::Var(v) = m {
                self.reject_protected(v.access, v.span);
                self.reject_protected(v.setter_access, v.span);
                // Computed properties are not stored fields.
                if v.computed.is_some() {
                    continue;
                }
                for b in &v.bindings {
                    let Some(fname) = pattern_ident_name(&b.pattern) else {
                        continue;
                    };
                    let ty = b
                        .ty
                        .as_ref()
                        .or_else(|| pattern_type(&b.pattern))
                        .map(|t| self.resolve_ast_type(t))
                        .unwrap_or(Ty::Error);
                    fields.insert(fname.to_string(), ty);
                    if let Some(sa) = v.setter_access {
                        setter_access.insert(fname.to_string(), sa);
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
                sealed_methods: vec![],
                is_abstract: false,
                is_sealed: false,
                is_actor: false,
                weak_fields: vec![],
                unowned_fields: vec![],
                computed_getters: IndexMap::new(),
                computed_setters: IndexMap::new(),
                will_set_observers: IndexMap::new(),
                did_set_observers: IndexMap::new(),
                setter_access,
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
        self.reject_protected(c.access, c.span);
        let name = c.name.clone();
        let mut fields = IndexMap::new();
        let mut weak_fields = Vec::new();
        let mut unowned_fields = Vec::new();
        for p in &c.primary_ctor {
            let mut ty = self.resolve_ast_type(&p.ty);
            if p.modifiers.weak {
                if !matches!(ty, Ty::Optional(_)) {
                    ty = Ty::optional(ty);
                }
                weak_fields.push(p.name.clone());
            } else if p.modifiers.unowned {
                unowned_fields.push(p.name.clone());
            }
            fields.insert(p.name.clone(), ty);
        }
        let mut setter_access = IndexMap::new();
        for m in &c.members {
            if let MemberDecl::Var(v) = m {
                self.reject_protected(v.access, v.span);
                self.reject_protected(v.setter_access, v.span);
                if v.computed.is_some() {
                    continue;
                }
                for b in &v.bindings {
                    let Some(fname) = pattern_ident_name(&b.pattern) else {
                        continue;
                    };
                    let mut ty = b
                        .ty
                        .as_ref()
                        .or_else(|| pattern_type(&b.pattern))
                        .map(|t| self.resolve_ast_type(t))
                        .unwrap_or(Ty::Error);
                    if v.modifiers.weak {
                        if !matches!(ty, Ty::Optional(_)) {
                            ty = Ty::optional(ty);
                        }
                        weak_fields.push(fname.to_string());
                    } else if v.modifiers.unowned {
                        unowned_fields.push(fname.to_string());
                    }
                    fields.insert(fname.to_string(), ty);
                    if let Some(sa) = v.setter_access {
                        setter_access.insert(fname.to_string(), sa);
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
            if let Some(pe) = self.types.get(p) {
                if pe.is_sealed {
                    self.error(
                        diag::LL2000,
                        format!("cannot inherit from sealed class `{p}`"),
                        c.span,
                    );
                }
            }
        }
        // Actors are Sendable by default (mailbox isolation).
        if c.is_actor && !conforms.iter().any(|x| x == "Sendable") {
            conforms.push("Sendable".into());
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
                sealed_methods: vec![],
                is_abstract: c.is_abstract,
                is_sealed: c.is_sealed,
                is_actor: c.is_actor,
                weak_fields,
                unowned_fields,
                computed_getters: IndexMap::new(),
                computed_setters: IndexMap::new(),
                will_set_observers: IndexMap::new(),
                did_set_observers: IndexMap::new(),
                setter_access,
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
                sealed_methods: vec![],
                is_abstract: false,
                is_sealed: false,
                is_actor: false,
                weak_fields: vec![],
                unowned_fields: vec![],
                    computed_getters: IndexMap::new(),
                    computed_setters: IndexMap::new(),
                    will_set_observers: IndexMap::new(),
                    did_set_observers: IndexMap::new(),
                    setter_access: IndexMap::new(),
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
                    is_async: false,
                    where_bounds: IndexMap::new(),
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
                sealed_methods: vec![],
                is_abstract: false,
                is_sealed: false,
                is_actor: false,
                weak_fields: vec![],
                unowned_fields: vec![],
                    computed_getters: IndexMap::new(),
                    computed_setters: IndexMap::new(),
                    will_set_observers: IndexMap::new(),
                    did_set_observers: IndexMap::new(),
                    setter_access: IndexMap::new(),
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
                    where_bounds: IndexMap::new(),
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
                Decl::Extension(e) => {
                    let Some(owner) = type_name_from_ast(&e.extended_type) else {
                        self.error(
                            diag::LL2000,
                            "extension target must be a named type",
                            e.span,
                        );
                        continue;
                    };
                    if !self.types.contains_key(&owner) {
                        // Builtins (String, etc.) — inject a Struct entry so methods attach.
                        if is_type_name(&owner) || matches!(
                            owner.as_str(),
                            "String" | "Int" | "Double" | "Bool" | "Array" | "Dictionary" | "Set"
                        ) {
                            self.types.entry(owner.clone()).or_insert_with(|| TypeEntry {
                                kind: TypeKind::Struct,
                                fields: IndexMap::new(),
                                methods: IndexMap::new(),
                                conforms: vec![],
                                parent: None,
                                enum_cases: IndexMap::new(),
                                protocol_methods: IndexMap::new(),
                                virtual_methods: vec![],
                                sealed_methods: vec![],
                                is_abstract: false,
                                is_sealed: false,
                                is_actor: false,
                                weak_fields: vec![],
                                unowned_fields: vec![],
                                    computed_getters: IndexMap::new(),
                                    computed_setters: IndexMap::new(),
                                    will_set_observers: IndexMap::new(),
                                    did_set_observers: IndexMap::new(),
                                    setter_access: IndexMap::new(),
                                });
                        } else {
                            self.error(
                                diag::LL2000,
                                format!("cannot extend unknown type `{owner}`"),
                                e.span,
                            );
                            continue;
                        }
                    }
                    let self_ty = match self.types.get(&owner).map(|t| t.kind) {
                        Some(TypeKind::Class) => Ty::Class(owner.clone()),
                        Some(TypeKind::Enum) => Ty::Enum(owner.clone()),
                        Some(TypeKind::Protocol) => Ty::Protocol(owner.clone()),
                        _ => Ty::Struct(owner.clone()),
                    };
                    // Protocol conformances added by extension.
                    let extra = Self::conforms_from_inheritance(&e.inheritance);
                    if let Some(entry) = self.types.get_mut(&owner) {
                        for c in extra {
                            if !entry.conforms.contains(&c) {
                                entry.conforms.push(c);
                            }
                        }
                    }
                    let where_bounds = self.extension_where_bounds(e);
                    self.collect_type_funcs_ext(
                        &owner,
                        self_ty,
                        &e.members,
                        &ns,
                        file,
                        where_bounds,
                    );
                }
                _ => {}
            }
        }
    }

    fn extension_where_bounds(
        &self,
        e: &lllang_syntax::ExtensionDecl,
    ) -> IndexMap<String, Vec<String>> {
        let mut out: IndexMap<String, Vec<String>> = IndexMap::new();
        for g in &e.generics {
            if let Some(ref c) = g.constraint {
                if let Some(n) = type_name_from_ast(c) {
                    out.entry(g.name.clone()).or_default().push(n);
                }
            }
        }
        for w in &e.where_clause {
            let Some(subject) = type_name_from_ast(&w.subject) else {
                continue;
            };
            let bound = match &w.bound {
                GenericBound::Type(t) | GenericBound::SameType(t) => type_name_from_ast(t),
            };
            if let Some(b) = bound {
                out.entry(subject).or_default().push(b);
            }
        }
        out
    }

    fn collect_type_funcs(
        &mut self,
        owner: &str,
        self_ty: Ty,
        members: &[MemberDecl],
        namespace: &str,
        file: &str,
    ) {
        self.collect_type_funcs_ext(owner, self_ty, members, namespace, file, IndexMap::new());
    }

    fn collect_type_funcs_ext(
        &mut self,
        owner: &str,
        self_ty: Ty,
        members: &[MemberDecl],
        namespace: &str,
        file: &str,
        where_bounds: IndexMap<String, Vec<String>>,
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
                    where_bounds: where_bounds.clone(),
                });
            } else if let MemberDecl::Var(v) = m {
                self.collect_computed_accessors(
                    owner,
                    &self_ty,
                    v,
                    namespace,
                    file,
                    &where_bounds,
                );
            }
        }
    }

    fn collect_computed_accessors(
        &mut self,
        owner: &str,
        self_ty: &Ty,
        v: &lllang_syntax::VarDecl,
        namespace: &str,
        file: &str,
        where_bounds: &IndexMap<String, Vec<String>>,
    ) {
        let Some(computed) = &v.computed else {
            // willSet/didSet observers: synthesize private methods hooked on assign later.
            if v.will_set.is_none() && v.did_set.is_none() {
                return;
            }
            for b in &v.bindings {
                let Some(fname) = pattern_ident_name(&b.pattern) else {
                    continue;
                };
                let field_ty = b
                    .ty
                    .clone()
                    .or_else(|| pattern_type(&b.pattern).cloned())
                    .unwrap_or_else(|| lllang_syntax::Type::Named {
                        span: v.span,
                        name: "Int".into(),
                        generics: vec![],
                    });
                if let Some(obs) = &v.will_set {
                    let gname = format!("__willSet_{fname}");
                    let pname = obs
                        .param_name
                        .clone()
                        .unwrap_or_else(|| "newValue".into());
                    let body = FuncBody::Block(obs.body.clone());
                    let decl = self.synth_method_decl(
                        v.span,
                        &gname,
                        Some((pname, field_ty.clone())),
                        None,
                        body,
                    );
                    if let Some(entry) = self.types.get_mut(owner) {
                        entry
                            .will_set_observers
                            .insert(fname.to_string(), gname.clone());
                    }
                    self.pending.push(PendingFunc {
                        decl,
                        owner: Some(owner.to_string()),
                        self_ty: Some(self_ty.clone()),
                        namespace: namespace.to_string(),
                        file: file.to_string(),
                        where_bounds: where_bounds.clone(),
                    });
                }
                if let Some(obs) = &v.did_set {
                    let gname = format!("__didSet_{fname}");
                    let pname = obs
                        .param_name
                        .clone()
                        .unwrap_or_else(|| "oldValue".into());
                    let body = FuncBody::Block(obs.body.clone());
                    let decl = self.synth_method_decl(
                        v.span,
                        &gname,
                        Some((pname, field_ty.clone())),
                        None,
                        body,
                    );
                    if let Some(entry) = self.types.get_mut(owner) {
                        entry
                            .did_set_observers
                            .insert(fname.to_string(), gname.clone());
                    }
                    self.pending.push(PendingFunc {
                        decl,
                        owner: Some(owner.to_string()),
                        self_ty: Some(self_ty.clone()),
                        namespace: namespace.to_string(),
                        file: file.to_string(),
                        where_bounds: where_bounds.clone(),
                    });
                }
            }
            return;
        };
        for b in &v.bindings {
            let Some(fname) = pattern_ident_name(&b.pattern) else {
                continue;
            };
            let get_name = format!("__get_{fname}");
            let get_body = match computed {
                lllang_syntax::ComputedBody::Arrow(e) => FuncBody::Expr(e.clone()),
                lllang_syntax::ComputedBody::Accessors { getter, .. } => {
                    FuncBody::Block(getter.clone())
                }
            };
            let ret_ty = b.ty.clone().or_else(|| pattern_type(&b.pattern).cloned());
            let get_decl =
                self.synth_method_decl(v.span, &get_name, None, ret_ty, get_body);
            if let Some(entry) = self.types.get_mut(owner) {
                entry
                    .computed_getters
                    .insert(fname.to_string(), get_name.clone());
            }
            self.pending.push(PendingFunc {
                decl: get_decl,
                owner: Some(owner.to_string()),
                self_ty: Some(self_ty.clone()),
                namespace: namespace.to_string(),
                file: file.to_string(),
                where_bounds: where_bounds.clone(),
            });
            if let lllang_syntax::ComputedBody::Accessors {
                setter: Some((_param, set_body)),
                ..
            } = computed
            {
                let set_name = format!("__set_{fname}");
                let set_ty = b
                    .ty
                    .clone()
                    .or_else(|| pattern_type(&b.pattern).cloned())
                    .unwrap_or_else(|| lllang_syntax::Type::Named {
                        span: v.span,
                        name: "Int".into(),
                        generics: vec![],
                    });
                let set_decl = self.synth_method_decl(
                    v.span,
                    &set_name,
                    Some(("newValue".into(), set_ty)),
                    None,
                    FuncBody::Block(set_body.clone()),
                );
                if let Some(entry) = self.types.get_mut(owner) {
                    entry
                        .computed_setters
                        .insert(fname.to_string(), set_name.clone());
                }
                self.pending.push(PendingFunc {
                    decl: set_decl,
                    owner: Some(owner.to_string()),
                    self_ty: Some(self_ty.clone()),
                    namespace: namespace.to_string(),
                    file: file.to_string(),
                    where_bounds: where_bounds.clone(),
                });
            }
        }
    }

    fn synth_method_decl(
        &self,
        span: Span,
        name: &str,
        param: Option<(String, lllang_syntax::Type)>,
        ret: Option<lllang_syntax::Type>,
        body: FuncBody,
    ) -> FuncDecl {
        let params = if let Some((pname, pty)) = param {
            vec![lllang_syntax::Param {
                span,
                external_name: None,
                internal_name: pname,
                ty: pty,
                default: None,
            }]
        } else {
            vec![]
        };
        FuncDecl {
            span,
            attrs: vec![],
            access: Some(lllang_syntax::Access::Public),
            is_async: false,
            is_override: false,
            is_virtual: false,
            is_abstract: false,
            is_sealed: false,
            name: name.to_string(),
            generics: vec![],
            params,
            return_type: ret,
            where_clause: vec![],
            throws: ThrowsClause {
                span: Span::EMPTY,
                types: vec![],
            },
            body,
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
            if !pf.decl.generics.is_empty() {
                self.active_type_params =
                    pf.decl.generics.iter().map(|g| g.name.clone()).collect();
            }
            let (params, _) = self.func_params(&pf.decl, pf.self_ty.clone());
            let ret = pf
                .decl
                .return_type
                .as_ref()
                .map(|t| self.resolve_ast_type(t))
                .unwrap_or(Ty::Void);
            self.active_type_params.clear();
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
                        is_async: pf.decl.is_async,
                    where_bounds: IndexMap::new(),
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
                                is_async: false,
                    where_bounds: IndexMap::new(),
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
                            is_async: false,
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
                    is_sealed: false,
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
        self.namespace = pf.namespace.clone();

        let deprecated_msg = Self::deprecated_message(&f.attrs);
        if let Some(ref msg) = deprecated_msg {
            let key = if let Some(ref owner) = pf.owner {
                format!("{owner}.{}", f.name)
            } else {
                f.name.clone()
            };
            self.deprecated.insert(key, msg.clone());
            self.deprecated.insert(f.name.clone(), msg.clone());
        }

        // `@extern(cName: "sym")` — register C linkage; no body emission.
        if pf.owner.is_none() {
            if let Some(cname) = Self::extern_cname_from_attrs(&f.attrs) {
                let (params, _) = self.func_params(f, None);
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
     is_async: false,
                    where_bounds: IndexMap::new(),
                };
                self.module_funcs
                    .insert(f.name.clone(), (sig, cname.clone()));
                self.extern_funcs.insert(f.name.clone(), cname);
                return;
            }
        }

        // Free generic functions: store as templates; specialize at call sites.
        if pf.owner.is_none() && !f.generics.is_empty() {
            let constraints = self.collect_generic_constraints(f);
            self.active_type_params = f.generics.iter().map(|g| g.name.clone()).collect();
            let (params, _) = self.func_params(f, None);
            let ret = f
                .return_type
                .as_ref()
                .map(|t| self.resolve_ast_type(t))
                .unwrap_or(Ty::Void);
            self.active_type_params.clear();
            let throws = throws_type(&f.throws);
            let sig = FuncSig {
     params: params.clone(),
     ret: ret.clone(),
     throws: throws.clone(),
     is_async: false,
                    where_bounds: IndexMap::new(),
                };
            let mangled = if !pf.namespace.is_empty() {
                format!("{}.{}", pf.namespace, f.name)
            } else {
                f.name.clone()
            };
            self.module_funcs
                .insert(f.name.clone(), (sig, mangled.clone()));
            self.generic_templates.insert(
                f.name.clone(),
                GenericTemplate {
                    decl: f.clone(),
                    namespace: pf.namespace.clone(),
                    file: pf.file.clone(),
                    constraints,
                },
            );
            return;
        }

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
            is_async: f.is_async,
            where_bounds: pf.where_bounds.clone(),
        };

        let mangled = if let Some(ref owner) = pf.owner {
            format!("{owner}.{}", f.name)
        } else if !pf.namespace.is_empty() {
            format!("{}.{}", pf.namespace, f.name)
        } else {
            f.name.clone()
        };

        // Enrich LSP symbol index with signature (incl. `throws`) and deprecation.
        let func_ty = Ty::Func {
            params: params.iter().map(|(_, t)| t.clone()).collect(),
            ret: Box::new(ret.clone()),
            throws: throws.as_ref().map(|t| Box::new(t.clone())),
        };
        self.index.annotate_global(
            &f.name,
            Some(&func_ty),
            deprecated_msg.as_deref(),
        );

        if pf.owner.is_none() {
            self.module_funcs.insert(f.name.clone(), (sig.clone(), mangled.clone()));
        } else if let Some(owner) = &pf.owner {
            self.check_override_rules(owner, f);
            let proto_needs_virt = {
                let conforms = self
                    .types
                    .get(owner)
                    .map(|e| e.conforms.clone())
                    .unwrap_or_default();
                let fname = f.name.as_str();
                conforms.iter().any(|p| {
                    self.types
                        .get(p)
                        .map(|pe| pe.protocol_methods.contains_key(fname))
                        .unwrap_or(false)
                })
            };
            if let Some(entry) = self.types.get_mut(owner) {
                entry.methods.insert(f.name.clone(), sig.clone());
                if f.is_virtual || f.is_override || proto_needs_virt {
                    if !entry.virtual_methods.contains(&f.name) {
                        entry.virtual_methods.push(f.name.clone());
                    }
                }
                if f.is_sealed {
                    entry.sealed_methods.push(f.name.clone());
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
            is_async: f.is_async,
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
        self.flattened_fields(name)
    }

    /// Ancestor fields (root → leaf) then own fields; later duplicates keep first (base) slot.
    fn flattened_fields(&self, class_name: &str) -> Vec<(String, Ty)> {
        let mut chain = Vec::new();
        let mut cur = Some(class_name.to_string());
        while let Some(n) = cur {
            let Some(e) = self.types.get(&n) else {
                break;
            };
            chain.push(n.clone());
            cur = e.parent.clone();
        }
        chain.reverse();
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for n in chain {
            if let Some(e) = self.types.get(&n) {
                for (k, v) in &e.fields {
                    if seen.insert(k.clone()) {
                        out.push((k.clone(), v.clone()));
                    }
                }
            }
        }
        out
    }

    fn merged_virtual_methods(&self, class_name: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = Some(class_name.to_string());
        while let Some(n) = cur {
            let Some(e) = self.types.get(&n) else {
                break;
            };
            chain.push(n.clone());
            cur = e.parent.clone();
        }
        chain.reverse();
        let mut slots: Vec<String> = Vec::new();
        for n in chain {
            if let Some(e) = self.types.get(&n) {
                for m in &e.virtual_methods {
                    if let Some(i) = slots.iter().position(|s| s == m) {
                        slots[i] = m.clone();
                    } else {
                        slots.push(m.clone());
                    }
                }
            }
        }
        slots
    }

    fn check_override_rules(&mut self, owner: &str, f: &FuncDecl) {
        let parent = self.types.get(owner).and_then(|e| e.parent.clone());
        let Some(pname) = parent else {
            if f.is_override {
                self.error(
                    diag::LL2000,
                    format!("`override` on `{}` but `{}` has no superclass", f.name, owner),
                    f.span,
                );
            }
            return;
        };
        // Walk parents for virtual / sealed.
        let mut found_virtual = false;
        let mut sealed = false;
        let mut cur = Some(pname.clone());
        while let Some(n) = cur {
            let Some(e) = self.types.get(&n) else {
                break;
            };
            if e.sealed_methods.iter().any(|m| m == &f.name) {
                sealed = true;
            }
            if e.virtual_methods.iter().any(|m| m == &f.name) {
                found_virtual = true;
            }
            if e.is_sealed && f.is_override {
                sealed = true;
            }
            cur = e.parent.clone();
        }
        if f.is_override && !found_virtual {
            self.error(
                diag::LL2000,
                format!("no virtual method `{}` to override in superclass of `{}`", f.name, owner),
                f.span,
            );
        }
        if f.is_override && sealed {
            self.error(
                diag::LL2000,
                format!("cannot override sealed method `{}`", f.name),
                f.span,
            );
        }
        if found_virtual && !f.is_override && !f.is_virtual {
            self.error(
                diag::LL2000,
                format!(
                    "method `{}` hides a virtual method; mark it `override` or rename",
                    f.name
                ),
                f.span,
            );
        }
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
                    let fields = self.flattened_fields(name);
                    let virtual_methods = self.merged_virtual_methods(name);
                    self.hir_types.push(TypeDef::Class {
                        name: name.clone(),
                        fields,
                        methods,
                        parent: entry.parent.clone(),
                        conforms: entry.conforms.clone(),
                        virtual_methods,
                        weak_fields: entry.weak_fields.clone(),
                        unowned_fields: entry.unowned_fields.clone(),
                        is_actor: entry.is_actor,
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
        let prev_async = self.in_async;
        let prev_lets = std::mem::take(&mut self.async_lets);
        let prev_joined = std::mem::take(&mut self.async_lets_joined);
        self.in_async = f.is_async;
        let ret = f
            .return_type
            .as_ref()
            .map(|t| self.resolve_ast_type(t))
            .unwrap_or(Ty::Void);
        let body = match &f.body {
            FuncBody::Block(b) => {
                let mut stmts =
                    self.check_block_stmts(&b.stmts, scope, self_ty.clone(), throws.clone());
                stmts.extend(self.async_let_cleanup_stmts());
                // `@loadEnv` — inject `Env.loadDotEnv(".env")` at function entry.
                if f.attrs.iter().any(|a| a.name == "loadEnv") {
                    stmts.insert(
                        0,
                        Stmt::Expr(Expr::Call {
                            callee: Callee::Builtin("env_load_dotenv".into()),
                            args: vec![Expr::StringLit(".env".into())],
                            ty: Ty::Void,
                            is_try: TryKind::None,
                        }),
                    );
                }
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
        };
        self.in_async = prev_async;
        self.async_lets = prev_lets;
        self.async_lets_joined = prev_joined;
        body
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
                    if l.is_async {
                        result.extend(self.check_async_let(
                            b,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                        ));
                        continue;
                    }
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
                    let init = self.coerce_to(init, &ty, scope, b.span);
                    let init_ty = self.expr_ty(&init, scope);
                    if !ty.is_error() && !self.ty_assignable(&ty, &init_ty) {
                        self.error_type_mismatch(&ty, &init_ty, b.span);
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
                    if let Expr::FuncRef {
                        name: mangled,
                        ty: fty,
                        captures,
                    } = &init
                    {
                        scope.insert_func_ref(
                            name.clone(),
                            fty.clone(),
                            mangled.clone(),
                            captures.clone(),
                        );
                    } else if expr_is_capture_literal(&init) {
                        scope.insert_literal(name.clone(), ty.clone(), init.clone());
                    } else {
                        scope.insert(name.clone(), ty.clone());
                    }
                    result.push(Stmt::Let {
                        name,
                        ty,
                        init,
                        async_let_result: None,
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
                    let init_expr = init_expr.map(|init| self.coerce_to(init, &ty, scope, b.span));
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
                let mut out = self.async_let_cleanup_stmts();
                // Mark pending async-lets joined so end-of-function cleanup skips them.
                for name in self.async_lets.keys().cloned().collect::<Vec<_>>() {
                    self.async_lets_joined.insert(name, ());
                }
                out.push(Stmt::Return(value.as_ref().map(|e| {
                    self.check_expr(e, scope, self_ty.clone(), throws.clone(), false)
                })));
                out
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
                            async_let_result: None,
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
            AStmt::For {
                pattern,
                sequence,
                body,
                span,
                ..
            } => {
                // `for (k, v) in dict` — desugar to keys foreach + dict_get.
                if let APat::Tuple { elements, .. } = pattern {
                    if elements.len() == 2 {
                        let k_name = match &elements[0] {
                            APat::Ident { name, .. } => name.clone(),
                            _ => {
                                self.error(
                                    diag::LL3001,
                                    "dictionary for-in keys must be a simple name",
                                    *span,
                                );
                                return vec![];
                            }
                        };
                        let v_name = match &elements[1] {
                            APat::Ident { name, .. } => name.clone(),
                            _ => {
                                self.error(
                                    diag::LL3001,
                                    "dictionary for-in values must be a simple name",
                                    *span,
                                );
                                return vec![];
                            }
                        };
                        let seq =
                            self.check_expr(sequence, scope, self_ty.clone(), throws.clone(), false);
                        let seq_ty = self.expr_ty(&seq, scope);
                        if !matches!(&seq_ty, Ty::Named(n) if n == "Dictionary") && !seq_ty.is_error()
                        {
                            self.error(
                                diag::LL2000,
                                format!(
                                    "`for (k, v)` expects a Dictionary, found `{}`",
                                    seq_ty.display()
                                ),
                                *span,
                            );
                        }
                        let keys = Expr::Call {
                            callee: Callee::Builtin("dict_keys".into()),
                            args: vec![seq.clone()],
                            ty: Ty::Array(Box::new(Ty::String)),
                            is_try: TryKind::None,
                        };
                        let mut loop_scope = scope.clone();
                        loop_scope.insert(k_name.clone(), Ty::String);
                        loop_scope.insert(v_name.clone(), Ty::String);
                        let mut body_stmts = vec![Stmt::Let {
                            name: v_name,
                            ty: Ty::String,
                            init: Expr::Call {
                                callee: Callee::Builtin("dict_get".into()),
                                args: vec![seq, Expr::Local(k_name.clone())],
                                ty: Ty::optional(Ty::String),
                                is_try: TryKind::None,
                            },
                            async_let_result: None,
                        }];
                        // Coerce Optional String → String via force path used elsewhere.
                        if let Stmt::Let { init, ty, .. } = &mut body_stmts[0] {
                            *init = Expr::Call {
                                callee: Callee::Builtin("unwrap_optional".into()),
                                args: vec![init.clone()],
                                ty: Ty::String,
                                is_try: TryKind::None,
                            };
                            *ty = Ty::String;
                        }
                        body_stmts.extend(self.check_block_stmts(
                            &body.stmts,
                            &mut loop_scope,
                            self_ty,
                            throws,
                        ));
                        return vec![Stmt::ForEach {
                            name: k_name,
                            array: keys,
                            body: Body { stmts: body_stmts },
                        }];
                    }
                }
                let name = match pattern {
                    APat::Ident { name, .. } => name.clone(),
                    _ => {
                        self.error(
                            diag::LL3001,
                            "for-loop patterns other than a simple name or `(k, v)` are not implemented yet",
                            *span,
                        );
                        return vec![];
                    }
                };
                // Range `a..<b` or `a...b`
                if let AExpr::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = sequence
                {
                    let start_e =
                        self.check_expr(start, scope, self_ty.clone(), throws.clone(), false);
                    let end_e = self.check_expr(end, scope, self_ty.clone(), throws.clone(), false);
                    let mut loop_scope = scope.clone();
                    loop_scope.insert(name.clone(), Ty::Int);
                    let body = Body {
                        stmts: self.check_block_stmts(
                            &body.stmts,
                            &mut loop_scope,
                            self_ty,
                            throws,
                        ),
                    };
                    return vec![Stmt::For {
                        name,
                        start: start_e,
                        end: end_e,
                        inclusive: *inclusive,
                        body,
                    }];
                }
                let seq = self.check_expr(sequence, scope, self_ty.clone(), throws.clone(), false);
                let seq_ty = self.expr_ty(&seq, scope);
                let elem_ty = match &seq_ty {
                    Ty::Array(inner) => (**inner).clone(),
                    _ => {
                        self.error(
                            diag::LL2000,
                            format!(
                                "for-in expects an array or range, found `{}`",
                                seq_ty.display()
                            ),
                            *span,
                        );
                        Ty::Error
                    }
                };
                let mut loop_scope = scope.clone();
                loop_scope.insert(name.clone(), elem_ty);
                let body = Body {
                    stmts: self.check_block_stmts(&body.stmts, &mut loop_scope, self_ty, throws),
                };
                vec![Stmt::ForEach {
                    name,
                    array: seq,
                    body,
                }]
            }
            AStmt::Break { .. } => vec![Stmt::Break],
            AStmt::Continue { .. } => vec![Stmt::Continue],
            AStmt::Expr { expr, .. } => {
                if let AExpr::Assign {
                    target,
                    value,
                    op,
                    span: aspan,
                } = expr
                {
                    let t = self.check_expr(target, scope, self_ty.clone(), throws.clone(), false);
                    let v = self.check_expr(value, scope, self_ty.clone(), throws.clone(), false);
                    let tty = self.expr_ty(&t, scope);
                    let vty = self.expr_ty(&v, scope);
                    // Desugar `x += y` → `x = x + y` (same for -= *= /= %=).
                    let v = if *op != AssignOp::Assign {
                        let bin = match op {
                            AssignOp::Add => BinaryOp::Add,
                            AssignOp::Sub => BinaryOp::Sub,
                            AssignOp::Mul => BinaryOp::Mul,
                            AssignOp::Div => BinaryOp::Div,
                            AssignOp::Mod => BinaryOp::Mod,
                            AssignOp::Assign => unreachable!(),
                        };
                        let result_ty = if matches!(tty, Ty::Double | Ty::Float)
                            || matches!(vty, Ty::Double | Ty::Float)
                        {
                            Ty::Double
                        } else {
                            tty.clone()
                        };
                        Expr::Binary {
                            op: bin,
                            left: Box::new(t.clone()),
                            right: Box::new(v),
                            ty: result_ty,
                        }
                    } else {
                        if !tty.is_error() && !vty.is_error() && !tty.assignable_from(&vty) {
                            self.error_type_mismatch(&tty, &vty, *aspan);
                        }
                        v
                    };
                    // Only locals, fields, and Int-array subscripts are assignable today.
                    match &t {
                        Expr::Local(_) | Expr::Param(_) | Expr::Member { .. } => {}
                        Expr::Index { .. } if tty == Ty::Int => {}
                        _ => {
                            self.error(
                                diag::LL3001,
                                "this assignment target is not implemented yet",
                                *aspan,
                            );
                            return vec![];
                        }
                    }
                    // Computed setter, observers, + `private(set)` enforcement.
                    if let Expr::Member { base, name, .. } = &t {
                        let base_ty = self.expr_ty(base, scope);
                        if let Some(tn) = type_name_key(&base_ty) {
                            let (setter_acc, set_name, will_name, did_name) = self
                                .types
                                .get(tn)
                                .map(|e| {
                                    (
                                        e.setter_access.get(name).copied(),
                                        e.computed_setters.get(name).cloned(),
                                        e.will_set_observers.get(name).cloned(),
                                        e.did_set_observers.get(name).cloned(),
                                    )
                                })
                                .unwrap_or((None, None, None, None));
                            if let Some(sa) = setter_acc {
                                let in_type = self_ty
                                    .as_ref()
                                    .and_then(|t| type_name_key(t).map(|s| s.to_string()))
                                    == Some(tn.to_string());
                                let same_file = self
                                    .types
                                    .get(tn)
                                    .map(|_| true) // fileprivate(set): same compilation unit MVP
                                    .unwrap_or(false);
                                let allowed = match sa {
                                    lllang_syntax::Access::Private => in_type,
                                    lllang_syntax::Access::Fileprivate => same_file,
                                    lllang_syntax::Access::Internal => true,
                                    _ => true,
                                };
                                if !allowed {
                                    self.error(
                                        diag::LL2000,
                                        format!(
                                            "setter of `{name}` is {sa:?}; cannot assign here"
                                        ),
                                        *aspan,
                                    );
                                }
                            }
                            if let Some(set_name) = set_name {
                                return vec![Stmt::Expr(Expr::Call {
                                    callee: Callee::Method {
                                        receiver: base.clone(),
                                        name: set_name,
                                    },
                                    args: vec![v],
                                    ty: Ty::Void,
                                    is_try: TryKind::None,
                                })];
                            }
                            if will_name.is_some() || did_name.is_some() {
                                let recv = (**base).clone();
                                let mut out = Vec::new();
                                let old = t.clone();
                                if let Some(wn) = will_name {
                                    out.push(Stmt::Expr(Expr::Call {
                                        callee: Callee::Method {
                                            receiver: Box::new(recv.clone()),
                                            name: wn,
                                        },
                                        args: vec![v.clone()],
                                        ty: Ty::Void,
                                        is_try: TryKind::None,
                                    }));
                                }
                                out.push(Stmt::Assign {
                                    target: t,
                                    value: v,
                                });
                                if let Some(dn) = did_name {
                                    out.push(Stmt::Expr(Expr::Call {
                                        callee: Callee::Method {
                                            receiver: Box::new(recv),
                                            name: dn,
                                        },
                                        args: vec![old],
                                        ty: Ty::Void,
                                        is_try: TryKind::None,
                                    }));
                                }
                                return out;
                            }
                        }
                    }
                    return vec![Stmt::Assign {
                        target: t,
                        value: v,
                    }];
                }
                vec![Stmt::Expr(self.check_expr(
                    expr,
                    scope,
                    self_ty,
                    throws.clone(),
                    false,
                ))]
            }
            AStmt::Block(b) => {
                let stmts = self.check_block_stmts(&b.stmts, scope, self_ty, throws);
                vec![Stmt::Scope {
                    body: Body { stmts },
                }]
            }
            AStmt::Defer { body, .. } => {
                let stmts = match body {
                    DeferBody::Block(b) => {
                        self.check_block_stmts(&b.stmts, scope, self_ty, throws)
                    }
                    DeferBody::Expr(e) => {
                        vec![Stmt::Expr(self.check_expr(
                            e,
                            scope,
                            self_ty,
                            throws,
                            false,
                        ))]
                    }
                };
                vec![Stmt::Defer {
                    body: Body { stmts },
                }]
            }
            AStmt::Die { message, .. } => vec![Stmt::Die {
                message: message.clone(),
            }],
            AStmt::DoCatch { body, catches, span } => {
                let mut body_stmts =
                    self.check_block_stmts(&body.stmts, scope, self_ty.clone(), throws.clone());
                if catches.is_empty() {
                    self.error(diag::LL2000, "`do` requires at least one `catch`", *span);
                    return body_stmts;
                }
                if let Some(Stmt::Expr(e)) = body_stmts.pop() {
                    // Multiple `catch` clauses: nest from last to first so each can handle.
                    let err_ty = throws.clone().unwrap_or(Ty::Named("Error".into()));
                    let mut nested = e;
                    let mut result_ty = Ty::Error;
                    for clause in catches.iter().rev() {
                        let binder = clause
                            .pattern
                            .as_ref()
                            .and_then(|p| match p {
                                APat::Ident { name, .. } => Some(name.clone()),
                                APat::Typed { pattern, ty, .. } => {
                                    // Typed catch: bind name if present; type is checked lightly.
                                    let _ = self.resolve_ast_type(ty);
                                    match pattern.as_ref() {
                                        APat::Ident { name, .. } => Some(name.clone()),
                                        _ => None,
                                    }
                                }
                                _ => None,
                            })
                            .unwrap_or_else(|| "e".into());
                        let mut arm_scope = scope.clone();
                        arm_scope.insert(binder.clone(), err_ty.clone());
                        let fallback = match &clause.body {
                            CatchBody::Arrow(ex) => self.check_expr(
                                ex,
                                &arm_scope,
                                self_ty.clone(),
                                throws.clone(),
                                true,
                            ),
                            CatchBody::Block(b) => {
                                let ss = self.check_block_stmts(
                                    &b.stmts,
                                    &mut arm_scope,
                                    self_ty.clone(),
                                    throws.clone(),
                                );
                                ss.into_iter()
                                    .find_map(|s| match s {
                                        Stmt::Expr(e) | Stmt::Return(Some(e)) => Some(e),
                                        _ => None,
                                    })
                                    .unwrap_or(Expr::Nil)
                            }
                        };
                        result_ty = type_of_expr(&fallback);
                        nested = Expr::Catch {
                            expr: Box::new(nested),
                            binder,
                            fallback: Box::new(fallback),
                            ty: result_ty.clone(),
                        };
                    }
                    let _ = result_ty;
                    body_stmts.push(Stmt::Expr(nested));
                }
                body_stmts
            }
            AStmt::Error { span } => {
                self.error(
                    diag::LL3001,
                    "this statement is not implemented yet",
                    *span,
                );
                vec![]
            }
            AStmt::CompileIf { span, .. } => {
                self.error(
                    diag::LL3001,
                    "`#if` should have been expanded before type checking",
                    *span,
                );
                vec![]
            }
            AStmt::Run { expr, span, .. } => {
                // `@run foo()` — zero-arg free (or type-mangled) void function; Sendable body.
                let checked =
                    self.check_expr(expr, scope, self_ty.clone(), throws.clone(), false);
                match checked {
                    Expr::Call {
                        callee: Callee::Func(name),
                        args,
                        ty,
                        ..
                    } if args.is_empty() => {
                        if !matches!(ty, Ty::Void) {
                            self.error(
                                diag::LL3001,
                                "`@run` requires a zero-argument `-> Void` function",
                                *span,
                            );
                            return vec![];
                        }
                        vec![Stmt::Run { func: name }]
                    }
                    Expr::Call {
                        callee: Callee::Method { receiver, name },
                        args,
                        ..
                    } if args.is_empty() => {
                        let recv_ty = self.expr_ty(&receiver, scope);
                        if !self.is_sendable(&recv_ty) {
                            self.error(
                                diag::LL2005,
                                format!(
                                    "`@run` receiver type `{}` is not Sendable",
                                    recv_ty.display()
                                ),
                                *span,
                            );
                        }
                        self.error(
                            diag::LL3001,
                            format!(
                                "`@run` cannot spawn instance method `{name}` yet — use a zero-argument free function"
                            ),
                            *span,
                        );
                        vec![]
                    }
                    _ => {
                        self.error(
                            diag::LL3001,
                            "`@run` requires a zero-argument free function call",
                            *span,
                        );
                        vec![]
                    }
                }
            }
            AStmt::AttrMacro { attr, .. } => {
                // Typecheck arguments; emission is handled by `lang test` desugar.
                for arg in &attr.args {
                    let _ = self.check_expr(arg, scope, self_ty.clone(), throws.clone(), false);
                }
                vec![]
            }
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
            // (name, ty, source, unwrap_optional)
            let mut if_let_bind: Option<(String, Ty, Expr, bool)> = None;
            let cond = match &arm.condition {
                IfCondition::Expr(e) => {
                    // `if x is T name` — bind `name: T` in the then branch when the check holds.
                    if let AExpr::Is {
                        expr,
                        ty: ast_ty,
                        binding: Some(bind_name),
                        ..
                    } = e
                    {
                        let inner =
                            self.check_expr(expr, scope, self_ty.clone(), throws.clone(), false);
                        let target = self.resolve_ast_type(ast_ty);
                        let from = self.expr_ty(&inner, scope);
                        let ok = target.assignable_from(&from)
                            || from.assignable_from(&target)
                            || numeric_cast_ok(&from, &target)
                            || self.ty_assignable(&target, &from);
                        if ok {
                            if_let_bind =
                                Some((bind_name.clone(), target.clone(), inner, false));
                            scope.insert(bind_name.clone(), target);
                        }
                        Expr::Bool(ok)
                    } else {
                        self.check_expr(e, scope, self_ty.clone(), throws.clone(), false)
                    }
                }
                IfCondition::Let(p, o) | IfCondition::Var(p, o) => {
                    let init = o
                        .as_ref()
                        .map(|e| self.check_expr(e, scope, self_ty.clone(), throws.clone(), false))
                        .unwrap_or(Expr::Bool(false));
                    let ety = self.expr_ty(&init, scope);
                    if let Ty::Optional(inner) = &ety {
                        if let APat::Ident { name, .. } = p {
                            if_let_bind =
                                Some((name.clone(), (**inner).clone(), init.clone(), true));
                            scope.insert(name.clone(), (**inner).clone());
                        }
                        Expr::Call {
                            callee: Callee::Builtin("optional_is_some".into()),
                            args: vec![init],
                            ty: Ty::Bool,
                            is_try: TryKind::None,
                        }
                    } else {
                        self.error(
                            diag::LL2002,
                            "`if let` requires an Optional value",
                            Span::EMPTY,
                        );
                        init
                    }
                }
            };
            let mut then_stmts = match &arm.body {
                IfBody::Block(b) => {
                    self.check_block_stmts(&b.stmts, scope, self_ty.clone(), throws.clone())
                }
                IfBody::Arrow(e) => {
                    vec![Stmt::Expr(self.check_expr(
                        e,
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        false,
                    ))]
                }
            };
            if let Some((name, ty, src, unwrap_opt)) = if_let_bind {
                let init = if unwrap_opt {
                    Expr::Call {
                        callee: Callee::Builtin("unwrap_optional".into()),
                        args: vec![src],
                        ty: ty.clone(),
                        is_try: TryKind::None,
                    }
                } else {
                    src
                };
                then_stmts.insert(
                    0,
                    Stmt::Let {
                        name,
                        ty: ty.clone(),
                        init,
                        async_let_result: None,
                    },
                );
            }
            let then_body = Body { stmts: then_stmts };
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
                SelfKind::Type => Expr::Self_,
                SelfKind::Super => Expr::Super,
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
            AExpr::Member {
                base,
                name,
                optional,
                ..
            } => {
                let b = self.check_expr(base, scope, self_ty.clone(), throws.clone(), throws_ok);
                if *optional {
                    self.check_optional_member(b, name, scope, self_ty)
                } else {
                    self.check_member(b, name, scope, self_ty)
                }
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
            AExpr::Try {
                expr,
                optional,
                force,
                ..
            } => {
                let inner = self.check_expr(
                    expr,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    true, // `try` / `try?` / `try!` all allow throwing callees
                );
                if *optional {
                    Expr::TryOptional(Box::new(inner))
                } else if *force {
                    match inner {
                        Expr::Call {
                            callee,
                            args,
                            ty,
                            ..
                        } => Expr::Call {
                            callee,
                            args,
                            ty,
                            is_try: TryKind::TryForce,
                        },
                        other => other,
                    }
                } else {
                    match inner {
                        Expr::Call {
                            callee,
                            args,
                            ty,
                            ..
                        } => Expr::Call {
                            callee,
                            args,
                            ty,
                            is_try: TryKind::Try,
                        },
                        other => other,
                    }
                }
            }
            // Honest await: join Task / async-let; spawn+join zero-arg async calls.
            AExpr::Await { expr, span, .. } => {
                if !self.in_async {
                    self.error(
                        diag::LL3001,
                        "`await` is only allowed inside an `async` function",
                        *span,
                    );
                }
                if let AExpr::Ident { name, .. } = expr.as_ref() {
                    if let Some(result_ty) = self.async_lets.get(name).cloned() {
                        self.async_lets_joined.insert(name.clone(), ());
                        let handle = Expr::Local(name.clone());
                        return if Self::ty_is_task_ret_ptr(&result_ty) {
                            Expr::Call {
                                callee: Callee::Builtin("task_join_ret".into()),
                                args: vec![handle],
                                ty: result_ty,
                                is_try: TryKind::None,
                            }
                        } else {
                            Expr::Call {
                                callee: Callee::Builtin("task_join".into()),
                                args: vec![handle],
                                ty: Ty::Void,
                                is_try: TryKind::None,
                            }
                        };
                    }
                }
                let inner = self.check_expr(expr, scope, self_ty, throws, throws_ok);
                self.lower_await(inner, *span)
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
            AExpr::Array { elements, span } => {
                if elements.is_empty() {
                    self.error(
                        diag::LL3001,
                        "empty array literals need an explicit type annotation",
                        *span,
                    );
                    return Expr::Nil;
                }
                let mut checked = Vec::new();
                let mut elem_ty = Ty::Error;
                for (i, el) in elements.iter().enumerate() {
                    let e = self.check_expr(el, scope, self_ty.clone(), throws.clone(), throws_ok);
                    let ty = self.expr_ty(&e, scope);
                    if i == 0 {
                        elem_ty = ty;
                    } else if !elem_ty.is_error()
                        && !ty.is_error()
                        && !elem_ty.assignable_from(&ty)
                    {
                        self.error_type_mismatch(&elem_ty, &ty, aexpr_span(el));
                    }
                    checked.push(e);
                }
                // Int uses i64 arrays; String and nominal/ref types use pointer arrays.
                let ok_elem = matches!(
                    &elem_ty,
                    Ty::Int
                        | Ty::String
                        | Ty::Named(_)
                        | Ty::Struct(_)
                        | Ty::Class(_)
                        | Ty::Enum(_)
                        | Ty::Result { .. }
                ) || elem_ty.is_error();
                if !ok_elem {
                    self.error(
                        diag::LL3001,
                        format!(
                            "array literal element type `{}` is not supported yet",
                            elem_ty.display()
                        ),
                        *span,
                    );
                }
                Expr::ArrayLit {
                    elems: checked,
                    elem_ty,
                }
            }
            AExpr::Dictionary { entries, span } => {
                let mut pairs = Vec::new();
                for (k, v) in entries {
                    let key = self.check_expr(k, scope, self_ty.clone(), throws.clone(), throws_ok);
                    let val = self.check_expr(v, scope, self_ty.clone(), throws.clone(), throws_ok);
                    let kt = self.expr_ty(&key, scope);
                    let vt = self.expr_ty(&val, scope);
                    if kt != Ty::String && !kt.is_error() {
                        self.error(
                            diag::LL3001,
                            "only `[String: String]` dictionary literals are implemented",
                            *span,
                        );
                    }
                    if vt != Ty::String && !vt.is_error() {
                        self.error(
                            diag::LL3001,
                            "only `[String: String]` dictionary literals are implemented",
                            *span,
                        );
                    }
                    pairs.push((key, val));
                }
                Expr::DictLit { entries: pairs }
            }
            AExpr::Index {
                base,
                indices,
                span,
                optional,
                ..
            } => {
                if indices.len() != 1 {
                    self.error(diag::LL3001, "multi-dimensional index is not implemented", *span);
                    return Expr::Nil;
                }
                let b = self.check_expr(base, scope, self_ty.clone(), throws.clone(), throws_ok);
                let i = self.check_expr(
                    &indices[0],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                if *optional {
                    return self.wrap_optional_chain(b, scope, |this, unwrapped, scope| {
                        let bty = this.expr_ty(&unwrapped, scope);
                        let elem_ty = match &bty {
                            Ty::Array(inner) => (**inner).clone(),
                            _ => {
                                this.error(
                                    diag::LL2000,
                                    format!("cannot index into `{}`", bty.display()),
                                    *span,
                                );
                                Ty::Error
                            }
                        };
                        (
                            Expr::Index {
                                base: Box::new(unwrapped),
                                index: Box::new(i.clone()),
                                ty: elem_ty.clone(),
                            },
                            elem_ty,
                        )
                    });
                }
                let bty = self.expr_ty(&b, scope);
                let elem_ty = match &bty {
                    Ty::Array(inner) => (**inner).clone(),
                    _ => {
                        self.error(
                            diag::LL2000,
                            format!("cannot index into `{}`", bty.display()),
                            *span,
                        );
                        Ty::Error
                    }
                };
                Expr::Index {
                    base: Box::new(b),
                    index: Box::new(i),
                    ty: elem_ty,
                }
            }
            AExpr::Lambda { params, body, span } => {
                self.check_lambda(params, body, *span, scope, self_ty, throws, throws_ok)
            }
            AExpr::Closure {
                params,
                body: ClosureBody::Expr(body),
                span,
                ..
            } => self.check_lambda(params, body, *span, scope, self_ty, throws, throws_ok),
            AExpr::Closure {
                params,
                body: ClosureBody::Block(block),
                span,
                ..
            } => {
                let mut ls = scope.clone();
                for (i, p) in params.iter().enumerate() {
                    let pname = p
                        .name
                        .clone()
                        .or_else(|| {
                            p.pattern.as_ref().and_then(|pat| match pat {
                                APat::Ident { name, .. } => Some(name.clone()),
                                _ => None,
                            })
                        })
                        .unwrap_or_else(|| format!("_{i}"));
                    let pty = p
                        .ty
                        .as_ref()
                        .map(|t| self.resolve_ast_type(t))
                        .unwrap_or(Ty::Int);
                    ls.insert(pname, pty);
                }
                let stmts =
                    self.check_block_stmts(&block.stmts, &mut ls, self_ty.clone(), throws.clone());
                let body_e = stmts
                    .into_iter()
                    .rev()
                    .find_map(|s| match s {
                        Stmt::Expr(e) | Stmt::Return(Some(e)) => Some(e),
                        _ => None,
                    })
                    .unwrap_or(Expr::Nil);
                self.finish_lambda(params, body_e, *span, &ls)
            }
            AExpr::Assign { span, .. } => {
                self.error(
                    diag::LL3001,
                    "assignment as an expression is not supported; use a statement",
                    *span,
                );
                Expr::Nil
            }
            AExpr::Unsafe { body, .. } => {
                self.unsafe_depth += 1;
                let out = match body {
                    lllang_syntax::UnsafeBody::Expr(e) => {
                        self.check_expr(e, scope, self_ty, throws, throws_ok)
                    }
                    lllang_syntax::UnsafeBody::Block(b) => {
                        let stmts =
                            self.check_block_stmts(&b.stmts, scope, self_ty, throws.clone());
                        stmts
                            .into_iter()
                            .rev()
                            .find_map(|s| match s {
                                Stmt::Expr(e) | Stmt::Return(Some(e)) => Some(e),
                                _ => None,
                            })
                            .unwrap_or(Expr::Nil)
                    }
                };
                self.unsafe_depth = self.unsafe_depth.saturating_sub(1);
                out
            }
            AExpr::NilCoalesce { left, right, span } => {
                let l = self.check_expr(left, scope, self_ty.clone(), throws.clone(), throws_ok);
                let r = self.check_expr(right, scope, self_ty.clone(), throws.clone(), throws_ok);
                let lty = self.expr_ty(&l, scope);
                let inner = match &lty {
                    Ty::Optional(t) => (**t).clone(),
                    _ => {
                        self.error(
                            diag::LL2002,
                            "`??` requires an Optional left-hand side",
                            *span,
                        );
                        self.expr_ty(&r, scope)
                    }
                };
                let cond = Expr::Call {
                    callee: Callee::Builtin("optional_is_some".into()),
                    args: vec![l.clone()],
                    ty: Ty::Bool,
                    is_try: TryKind::None,
                };
                let then_e = Expr::Call {
                    callee: Callee::Builtin("unwrap_optional".into()),
                    args: vec![l],
                    ty: inner.clone(),
                    is_try: TryKind::None,
                };
                Expr::IfExpr {
                    cond: Box::new(cond),
                    then_e: Box::new(then_e),
                    else_e: Box::new(r),
                    ty: inner,
                }
            }
            AExpr::ForceUnwrap { expr, span } => {
                let inner = self.check_expr(expr, scope, self_ty, throws, throws_ok);
                let ty = self.expr_ty(&inner, scope);
                match ty {
                    Ty::Optional(inner_ty) => Expr::Call {
                        callee: Callee::Builtin("unwrap_optional".into()),
                        args: vec![inner],
                        ty: *inner_ty,
                        is_try: TryKind::None,
                    },
                    _ => {
                        self.error(
                            diag::LL2002,
                            "force-unwrap `!` requires an Optional value",
                            *span,
                        );
                        inner
                    }
                }
            }
            AExpr::Is {
                expr,
                ty: ast_ty,
                binding,
                span,
            } => {
                let _ = binding;
                let inner = self.check_expr(expr, scope, self_ty.clone(), throws.clone(), throws_ok);
                let target = self.resolve_ast_type(ast_ty);
                let from = self.expr_ty(&inner, scope);
                // Classes / ARC objects: runtime `ll_isa`. Scalars: static compatibility.
                if matches!(from, Ty::Class(_)) || matches!(target, Ty::Class(_)) {
                    let type_name = match &target {
                        Ty::Class(n) | Ty::Struct(n) | Ty::Enum(n) => n.clone(),
                        Ty::Named(n) => n.clone(),
                        _ => {
                            self.error(
                                diag::LL3001,
                                format!("`is {}` is not supported for this type yet", target.display()),
                                *span,
                            );
                            return Expr::Bool(false);
                        }
                    };
                    return Expr::Call {
                        callee: Callee::Builtin("ll_isa".into()),
                        args: vec![inner, Expr::StringLit(type_name)],
                        ty: Ty::Bool,
                        is_try: TryKind::None,
                    };
                }
                let ok = target.assignable_from(&from)
                    || from.assignable_from(&target)
                    || numeric_cast_ok(&from, &target)
                    || self.ty_assignable(&target, &from);
                Expr::Bool(ok)
            }
            AExpr::Cast {
                expr,
                ty: ast_ty,
                kind,
                span,
            } => {
                let inner = self.check_expr(expr, scope, self_ty, throws, throws_ok);
                let to = self.resolve_ast_type(ast_ty);
                let from = self.expr_ty(&inner, scope);
                match kind {
                    lllang_syntax::CastKind::As => {
                        if !to.is_error()
                            && !from.is_error()
                            && !to.assignable_from(&from)
                            && !from.assignable_from(&to)
                            && !numeric_cast_ok(&from, &to)
                        {
                            self.error(
                                diag::LL2002,
                                format!(
                                    "cannot cast `{}` to `{}`",
                                    from.display(),
                                    to.display()
                                ),
                                *span,
                            );
                        }
                        Expr::Cast {
                            expr: Box::new(inner),
                            to,
                        }
                    }
                    lllang_syntax::CastKind::AsOptional => {
                        if matches!(from, Ty::Class(_)) || matches!(to, Ty::Class(_)) {
                            let type_name = match &to {
                                Ty::Class(n) | Ty::Struct(n) => n.clone(),
                                Ty::Named(n) => n.clone(),
                                _ => String::new(),
                            };
                            let check = Expr::Call {
                                callee: Callee::Builtin("ll_isa".into()),
                                args: vec![inner.clone(), Expr::StringLit(type_name)],
                                ty: Ty::Bool,
                                is_try: TryKind::None,
                            };
                            let some = Expr::OptionalSome(Box::new(Expr::Cast {
                                expr: Box::new(inner),
                                to: to.clone(),
                            }));
                            let none = Expr::OptionalNone(to.clone());
                            return Expr::IfExpr {
                                cond: Box::new(check),
                                then_e: Box::new(some),
                                else_e: Box::new(none),
                                ty: Ty::optional(to),
                            };
                        }
                        // Numeric / static: succeed into Optional(to).
                        Expr::OptionalSome(Box::new(Expr::Cast {
                            expr: Box::new(inner),
                            to,
                        }))
                    }
                    lllang_syntax::CastKind::AsForced => {
                        if matches!(from, Ty::Class(_)) || matches!(to, Ty::Class(_)) {
                            let type_name = match &to {
                                Ty::Class(n) | Ty::Struct(n) => n.clone(),
                                Ty::Named(n) => n.clone(),
                                _ => String::new(),
                            };
                            let check = Expr::Call {
                                callee: Callee::Builtin("ll_isa".into()),
                                args: vec![inner.clone(), Expr::StringLit(type_name)],
                                ty: Ty::Bool,
                                is_try: TryKind::None,
                            };
                            let ok = Expr::Cast {
                                expr: Box::new(inner),
                                to: to.clone(),
                            };
                            let trap = Expr::Call {
                                callee: Callee::Builtin("ll_trap".into()),
                                args: vec![
                                    Expr::StringLit("cast".into()),
                                    Expr::StringLit(format!("as! {} failed", to.display())),
                                ],
                                ty: to.clone(),
                                is_try: TryKind::None,
                            };
                            return Expr::IfExpr {
                                cond: Box::new(check),
                                then_e: Box::new(ok),
                                else_e: Box::new(trap),
                                ty: to,
                            };
                        }
                        Expr::Cast {
                            expr: Box::new(inner),
                            to,
                        }
                    }
                }
            }
            other => {
                self.error(
                    diag::LL3001,
                    format!(
                        "expression form `{}` is not implemented yet",
                        aexpr_kind(other)
                    ),
                    aexpr_span(other),
                );
                Expr::Nil
            }
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
        // `PI` math constant (Std.Math).
        if name == "PI" {
            return Expr::Float(std::f64::consts::PI);
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
                return Expr::OptionalNone(Ty::Error);
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
            // Ctor sugar before empty struct construct: `CancellationToken()`.
            if name == "CancellationToken" {
                if !args.is_empty() {
                    self.error(
                        diag::LL2000,
                        "`CancellationToken()` takes no arguments",
                        span,
                    );
                }
                return Expr::Call {
                    callee: Callee::Builtin("cancellation_token_create".into()),
                    args: vec![],
                    ty: Ty::Named("CancellationToken".into()),
                    is_try: TryKind::None,
                };
            }
            if name == "TaskGroup" {
                if !args.is_empty() {
                    self.error(
                        diag::LL2000,
                        "`TaskGroup()` takes no arguments",
                        span,
                    );
                }
                return Expr::Call {
                    callee: Callee::Builtin("task_group_create".into()),
                    args: vec![],
                    ty: Ty::Named("TaskGroup".into()),
                    is_try: TryKind::None,
                };
            }
            if let Some(entry) = self.types.get(name) {
                if matches!(entry.kind, TypeKind::Struct | TypeKind::Class) {
                    return self.check_construct(name, args, scope, self_ty, throws, throws_ok);
                }
            }
            if name == "Decimal" {
                let checked = self.check_args_against(
                    args,
                    &[Ty::String],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                return Expr::Call {
                    callee: Callee::Builtin("decimal_from_string".into()),
                    args: checked,
                    ty: Ty::Named("Decimal".into()),
                    is_try: TryKind::None,
                };
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
                } else if name == "clamp" || name == "max" || name == "min" {
                    // Accept labeled args (`min:`, `max:`) by position order.
                    args.iter()
                        .map(|a| {
                            self.check_expr(
                                &a.value,
                                scope,
                                self_ty.clone(),
                                throws.clone(),
                                throws_ok,
                            )
                        })
                        .collect()
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

        if let AExpr::Member {
            base,
            name: method,
            optional: opt_chain,
            ..
        } = callee
        {
            if let AExpr::Ident { name: type_name, .. } = base.as_ref() {
                if is_type_name(type_name) || self.types.contains_key(type_name) {
                    if type_name == "Env" || type_name == "Environment" {
                        if let Some(call) = self.check_env_call(
                            method,
                            args,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                            try_kind,
                        ) {
                            return call;
                        }
                    }
                    // HttpServer.serve(port, handle) — durable accept loop.
                    if type_name == "HttpServer" && method == "serve" {
                        if args.len() != 2 {
                            self.error(
                                diag::LL2000,
                                "`HttpServer.serve` expects `(port: Int, handler: (String) -> String)`",
                                span,
                            );
                            return Expr::Nil;
                        }
                        let port = self.check_expr(
                            &args[0].value,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let handler = self.check_expr(
                            &args[1].value,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        if let Some(fname) =
                            self.resolve_string_to_string_func(&handler, scope)
                        {
                            return Expr::Call {
                                callee: Callee::HttpServe { handler: fname },
                                args: vec![port],
                                ty: Ty::Void,
                                is_try: TryKind::None,
                            };
                        }
                        self.error(
                            diag::LL3001,
                            "`HttpServer.serve` requires a `(String) -> String` function reference",
                            span,
                        );
                        return Expr::Nil;
                    }
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
                    // Enum case construction: `Color.blue(7)`
                    if let Some(entry) = self.types.get(type_name) {
                        if matches!(entry.kind, TypeKind::Enum) {
                            if let Some(case_info) = entry.enum_cases.get(method) {
                                let params: Vec<Ty> =
                                    case_info.fields.iter().map(|(_, t)| t.clone()).collect();
                                let checked = self.check_args_against(
                                    args,
                                    &params,
                                    scope,
                                    self_ty.clone(),
                                    throws.clone(),
                                    throws_ok,
                                );
                                return Expr::EnumCase {
                                    enum_name: type_name.clone(),
                                    case: method.clone(),
                                    args: checked,
                                    ty: Ty::Enum(type_name.clone()),
                                };
                            }
                        }
                    }
                }
            }
            let raw_base =
                self.check_expr(base, scope, self_ty.clone(), throws.clone(), throws_ok);
            let (base_expr, opt_chain_src) = if *opt_chain {
                let bty = self.expr_ty(&raw_base, scope);
                match bty {
                    Ty::Optional(inner) => {
                        let unwrapped = Expr::Call {
                            callee: Callee::Builtin("unwrap_optional".into()),
                            args: vec![raw_base.clone()],
                            ty: (*inner).clone(),
                            is_try: TryKind::None,
                        };
                        (unwrapped, Some(raw_base))
                    }
                    _ => {
                        self.error(
                            diag::LL2002,
                            "optional chaining `?.` requires an Optional receiver",
                            span,
                        );
                        (raw_base, None)
                    }
                }
            } else {
                (raw_base, None)
            };
            let wrap_opt_chain = |call: Expr| -> Expr {
                if let Some(ref src) = opt_chain_src {
                    let ret = type_of_expr(&call);
                    let opt_ty = Ty::Optional(Box::new(ret.clone()));
                    Expr::IfExpr {
                        cond: Box::new(Expr::Call {
                            callee: Callee::Builtin("optional_is_some".into()),
                            args: vec![src.clone()],
                            ty: Ty::Bool,
                            is_try: TryKind::None,
                        }),
                        then_e: Box::new(Expr::OptionalSome(Box::new(call))),
                        else_e: Box::new(Expr::OptionalNone(ret)),
                        ty: opt_ty,
                    }
                } else {
                    call
                }
            };
            // `super.method(...)` → static parent dispatch.
            if matches!(base_expr, Expr::Super) {
                let parent = match &self_ty {
                    Some(Ty::Class(n)) => self.types.get(n).and_then(|e| e.parent.clone()),
                    _ => None,
                };
                let Some(pname) = parent else {
                    self.error(diag::LL2000, "`super` requires a superclass", span);
                    return Expr::Nil;
                };
                let (sig, _) = self
                    .lookup_method(&Ty::Class(pname.clone()), method)
                    .unwrap_or((
                        FuncSig {
                            params: vec![],
                            ret: Ty::Error,
                            throws: None,
                            is_async: false,
                    where_bounds: IndexMap::new(),
                },
                        false,
                    ));
                let checked = self.check_call_args(
                    args,
                    &sig.params,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                return Expr::Call {
                    callee: Callee::SuperMethod {
                        parent: pname,
                        name: method.clone(),
                    },
                    args: checked,
                    ty: sig.ret,
                    is_try: map_try(try_kind),
                };
            }
            let base_ty = self.expr_ty(&base_expr, scope);
            if base_ty == Ty::String && method == "contains" {
                let checked = self.check_args_against(
                    args,
                    &[Ty::String],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                let mut call_args = vec![base_expr];
                call_args.extend(checked);
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_contains".into()),
                    args: call_args,
                    ty: Ty::Bool,
                    is_try: TryKind::None,
                });
            }
            if base_ty == Ty::String && method == "trim" {
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_trim".into()),
                    args: vec![base_expr],
                    ty: Ty::String,
                    is_try: TryKind::None,
                });
            }
            if base_ty == Ty::String && method == "substring" {
                let checked = self.check_args_against(
                    args,
                    &[Ty::Int, Ty::Int],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                let mut call_args = vec![base_expr];
                call_args.extend(checked);
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_substring".into()),
                    args: call_args,
                    ty: Ty::String,
                    is_try: TryKind::None,
                });
            }
            if base_ty == Ty::String && method == "split" {
                let checked = self.check_args_against(
                    args,
                    &[Ty::String],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                let mut call_args = vec![base_expr];
                call_args.extend(checked);
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_split".into()),
                    args: call_args,
                    ty: Ty::Array(Box::new(Ty::String)),
                    is_try: TryKind::None,
                });
            }
            // TaskGroup.add(worker) — zero-arg void free function / FuncRef.
            if matches!(&base_ty, Ty::Named(n) if n == "TaskGroup") && method == "add" {
                if args.len() != 1 {
                    self.error(
                        diag::LL2000,
                        "`TaskGroup.add` expects one zero-argument `-> Void` function",
                        span,
                    );
                    return Expr::Call {
                        callee: Callee::Builtin("ll_noop".into()),
                        args: vec![base_expr],
                        ty: Ty::Void,
                        is_try: TryKind::None,
                    };
                }
                let worker = self.check_expr(
                    &args[0].value,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                if let Some(fname) = self.resolve_void_zero_arg_func(&worker, scope) {
                    return Expr::Call {
                        callee: Callee::TaskGroupAdd { name: fname },
                        args: vec![base_expr],
                        ty: Ty::Void,
                        is_try: TryKind::None,
                    };
                }
                self.error(
                    diag::LL3001,
                    "`TaskGroup.add` requires a zero-argument free `-> Void` function reference",
                    span,
                );
                return Expr::Call {
                    callee: Callee::Builtin("ll_noop".into()),
                    args: vec![base_expr],
                    ty: Ty::Void,
                    is_try: TryKind::None,
                };
            }
            // String.matches(pattern) → Regex.isMatch(pattern, self)
            if base_ty == Ty::String && method == "matches" {
                let checked = self.check_args_against(
                    args,
                    &[Ty::String],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                let mut call_args = checked;
                call_args.push(base_expr);
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("regex_is_match".into()),
                    args: call_args,
                    ty: Ty::Bool,
                    is_try: TryKind::None,
                });
            }
            if base_ty == Ty::String && method == "uppercased" {
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_upper".into()),
                    args: vec![base_expr],
                    ty: Ty::String,
                    is_try: TryKind::None,
                });
            }
            if base_ty == Ty::String && method == "lowercased" {
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_lower".into()),
                    args: vec![base_expr],
                    ty: Ty::String,
                    is_try: TryKind::None,
                });
            }
            if base_ty == Ty::String && method == "replacing" {
                let checked: Vec<_> = args
                    .iter()
                    .map(|a| {
                        self.check_expr(
                            &a.value,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        )
                    })
                    .collect();
                let mut call_args = vec![base_expr];
                call_args.extend(checked);
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_replace".into()),
                    args: call_args,
                    ty: Ty::String,
                    is_try: TryKind::None,
                });
            }
            if matches!(&base_ty, Ty::Array(_)) && method == "joined" {
                let checked: Vec<_> = args
                    .iter()
                    .map(|a| {
                        self.check_expr(
                            &a.value,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        )
                    })
                    .collect();
                let mut call_args = vec![base_expr];
                call_args.extend(checked);
                return wrap_opt_chain(Expr::Call {
                    callee: Callee::Builtin("string_join".into()),
                    args: call_args,
                    ty: Ty::String,
                    is_try: TryKind::None,
                });
            }
            if matches!(&base_ty, Ty::Named(n) if n == "Dictionary") {
                match method.as_str() {
                    "set" => {
                        let checked = self.check_args_against(
                            args,
                            &[Ty::String, Ty::String],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let mut call_args = vec![base_expr];
                        call_args.extend(checked);
                        return Expr::Call {
                            callee: Callee::Builtin("dict_set".into()),
                            args: call_args,
                            ty: Ty::Void,
                            is_try: TryKind::None,
                        };
                    }
                    "get" => {
                        let checked = self.check_args_against(
                            args,
                            &[Ty::String],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let mut call_args = vec![base_expr];
                        call_args.extend(checked);
                        return Expr::Call {
                            callee: Callee::Builtin("dict_get".into()),
                            args: call_args,
                            ty: Ty::optional(Ty::String),
                            is_try: TryKind::None,
                        };
                    }
                    "contains" => {
                        let checked = self.check_args_against(
                            args,
                            &[Ty::String],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let mut call_args = vec![base_expr];
                        call_args.extend(checked);
                        return Expr::Call {
                            callee: Callee::Builtin("dict_contains".into()),
                            args: call_args,
                            ty: Ty::Bool,
                            is_try: TryKind::None,
                        };
                    }
                    _ => {}
                }
            }
            if matches!(&base_ty, Ty::Named(n) if n == "Set") {
                match method.as_str() {
                    "insert" => {
                        let checked = self.check_args_against(
                            args,
                            &[Ty::String],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let mut call_args = vec![base_expr];
                        call_args.extend(checked);
                        return Expr::Call {
                            callee: Callee::Builtin("set_insert".into()),
                            args: call_args,
                            ty: Ty::Void,
                            is_try: TryKind::None,
                        };
                    }
                    "contains" => {
                        let checked = self.check_args_against(
                            args,
                            &[Ty::String],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let mut call_args = vec![base_expr];
                        call_args.extend(checked);
                        return Expr::Call {
                            callee: Callee::Builtin("set_contains".into()),
                            args: call_args,
                            ty: Ty::Bool,
                            is_try: TryKind::None,
                        };
                    }
                    "union" | "intersection" | "difference" => {
                        let checked = self.check_args_against(
                            args,
                            &[Ty::Named("Set".into())],
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                        );
                        let mut call_args = vec![base_expr];
                        call_args.extend(checked);
                        return Expr::Call {
                            callee: Callee::Builtin(format!("set_{method}")),
                            args: call_args,
                            ty: Ty::Named("Set".into()),
                            is_try: TryKind::None,
                        };
                    }
                    _ => {}
                }
            }
            if matches!(base_ty, Ty::Array(ref inner) if **inner == Ty::Int) && method == "append" {
                let checked = self.check_args_against(
                    args,
                    &[Ty::Int],
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                let mut call_args = vec![base_expr];
                call_args.extend(checked);
                return Expr::Call {
                    callee: Callee::Builtin("array_append_i64".into()),
                    args: call_args,
                    ty: Ty::Void,
                    is_try: TryKind::None,
                };
            }
            // Phase 1.2: [Int] HOFs — desugar to a synthesized helper that loops + calls the lambda.
            if matches!(&base_ty, Ty::Array(inner) if **inner == Ty::Int) {
                match method.as_str() {
                    "map" | "filter" | "sorted" | "any" | "all" | "reduce" => {
                        if let Some(out) = self.check_int_array_hof(
                            method,
                            base_expr.clone(),
                            args,
                            scope,
                            self_ty.clone(),
                            throws.clone(),
                            throws_ok,
                            span,
                        ) {
                            return wrap_opt_chain(out);
                        }
                    }
                    _ => {}
                }
            }
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
                        for a in args {
                            checked.push(self.check_expr(
                                &a.value,
                                scope,
                                self_ty.clone(),
                                throws.clone(),
                                throws_ok,
                            ));
                        }
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
                self.check_actor_isolation(&base_ty, &checked, span, scope);
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
                return wrap_opt_chain(Expr::Call {
                    callee,
                    args: checked,
                    ty: sig.ret.clone(),
                    is_try: map_try(try_kind),
                });
            }
        }

        let callee_expr = self.check_expr(callee, scope, self_ty.clone(), throws.clone(), throws_ok);
        if let Expr::FuncRef {
            name,
            ty,
            captures,
        } = &callee_expr
        {
            if let Ty::Func {
                params,
                ret,
                throws: thr,
            } = ty
            {
                let checked = self.check_args_against(
                    args,
                    params,
                    scope,
                    self_ty.clone(),
                    throws.clone(),
                    throws_ok,
                );
                self.verify_try(
                    &thr.as_ref().map(|t| (**t).clone()),
                    try_kind,
                    throws_ok,
                    span,
                );
                let mut full_args = captures.clone();
                full_args.extend(checked);
                return Expr::Call {
                    callee: Callee::Func(name.clone()),
                    args: full_args,
                    ty: (**ret).clone(),
                    is_try: map_try(try_kind),
                };
            }
        }
        if let Expr::Local(ref n) = callee_expr {
            self.note_deprecated_use(n, span);
            if let Some((mangled, caps)) = scope.func_ref(n).map(|(m, c)| (m.to_string(), c.to_vec()))
            {
                let found = self.module_funcs.get(&mangled).cloned().or_else(|| {
                    let short = mangled.rsplit('.').next().unwrap_or(&mangled);
                    self.module_funcs.get(short).cloned()
                }).or_else(|| {
                    self.module_funcs
                        .iter()
                        .find(|(_, (_, m))| m == &mangled)
                        .map(|(_, v)| v.clone())
                });
                if let Some((sig, real_name)) = found {
                    // User-facing params exclude leading capture params already in `caps`.
                    let user_params: Vec<(String, Ty)> = sig
                        .params
                        .iter()
                        .skip(caps.len())
                        .cloned()
                        .collect();
                    let checked = self.check_call_args(
                        args,
                        &user_params,
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        throws_ok,
                    );
                    self.verify_try(&sig.throws, try_kind, throws_ok, span);
                    let mut full_args = caps;
                    full_args.extend(checked);
                    return Expr::Call {
                        callee: Callee::Func(real_name),
                        args: full_args,
                        ty: sig.ret,
                        is_try: map_try(try_kind),
                    };
                }
            }
            if let Some(cname) = self.extern_funcs.get(n).cloned() {
                if self.unsafe_depth == 0 {
                    self.error(
                        diag::LL2002,
                        format!("call to `@extern` function `{n}` requires an `unsafe` block"),
                        span,
                    );
                }
                if let Some((sig, _)) = self.module_funcs.get(n).cloned() {
                    let checked = self.check_call_args(
                        args,
                        &sig.params,
                        scope,
                        self_ty.clone(),
                        throws.clone(),
                        throws_ok,
                    );
                    return Expr::Call {
                        callee: Callee::Builtin(cname),
                        args: checked,
                        ty: sig.ret,
                        is_try: map_try(try_kind),
                    };
                }
            }
            if self.generic_templates.contains_key(n) {
                let checked: Vec<Expr> = args
                    .iter()
                    .map(|a| {
                        self.check_expr(&a.value, scope, self_ty.clone(), throws.clone(), throws_ok)
                    })
                    .collect();
                let arg_tys: Vec<Ty> = checked.iter().map(|e| self.expr_ty(e, scope)).collect();
                if let Some((mangled, sig)) = self.specialize_generic_call(n, &arg_tys, span) {
                    self.verify_try(&sig.throws, try_kind, throws_ok, span);
                    return Expr::Call {
                        callee: Callee::Func(mangled),
                        args: checked,
                        ty: sig.ret,
                        is_try: map_try(try_kind),
                    };
                }
                return Expr::Nil;
            }
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
        if matches!(entry.as_ref().map(|e| e.is_abstract), Some(true)) {
            self.error(
                diag::LL2000,
                format!("cannot construct abstract class `{type_name}`"),
                Span::EMPTY,
            );
        }
        let ty = match entry.as_ref().map(|e| &e.kind) {
            Some(TypeKind::Struct) => Ty::Struct(type_name.to_string()),
            Some(TypeKind::Class) => Ty::Class(type_name.to_string()),
            _ => Ty::Error,
        };
        let field_order: Vec<_> = if matches!(ty, Ty::Class(_)) {
            self.flattened_fields(type_name)
                .into_iter()
                .map(|(n, _)| n)
                .collect()
        } else {
            entry
                .map(|e| e.fields.keys().cloned().collect())
                .unwrap_or_default()
        };
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

    fn check_lambda(
        &mut self,
        params: &[ClosureParam],
        body: &AExpr,
        span: Span,
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
    ) -> Expr {
        let mut ls = scope.clone();
        let mut sig_params = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let pname = p
                .name
                .clone()
                .or_else(|| {
                    p.pattern.as_ref().and_then(|pat| match pat {
                        APat::Ident { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_else(|| if i == 0 { "x".into() } else { format!("_{i}") });
            let pty = p
                .ty
                .as_ref()
                .map(|t| self.resolve_ast_type(t))
                .unwrap_or(Ty::Int);
            ls.insert(pname.clone(), pty.clone());
            sig_params.push((pname, pty));
        }
        if sig_params.is_empty() {
            // Zero-arg closure.
        }
        let body_e = self.check_expr(body, &ls, self_ty.clone(), throws.clone(), throws_ok);
        self.finish_lambda_checked(sig_params, body_e, span, &ls)
    }

    fn finish_lambda(
        &mut self,
        params: &[ClosureParam],
        body_e: Expr,
        span: Span,
        scope: &Scope,
    ) -> Expr {
        let mut sig_params = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let pname = p
                .name
                .clone()
                .or_else(|| {
                    p.pattern.as_ref().and_then(|pat| match pat {
                        APat::Ident { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_else(|| if i == 0 { "x".into() } else { format!("_{i}") });
            let pty = scope.get(&pname).unwrap_or(Ty::Int);
            let _ = span;
            sig_params.push((pname, pty));
        }
        self.finish_lambda_checked(sig_params, body_e, span, scope)
    }

    fn finish_lambda_checked(
        &mut self,
        sig_params: Vec<(String, Ty)>,
        mut body_e: Expr,
        span: Span,
        scope: &Scope,
    ) -> Expr {
        let param_names: std::collections::HashSet<String> =
            sig_params.iter().map(|(n, _)| n.clone()).collect();
        let mut free = std::collections::BTreeSet::new();
        collect_free_locals(&body_e, &param_names, &mut free);

        // Value captures: prepend as parameters; at the FuncRef site emit a bound
        // wrapper is fragile across functions. Instead, rewrite free Locals that are
        // Int/Bool/… into leading Params of the lambda and require the call path to
        // pass them — we do that by synthesizing a capturing call thunk stored as
        // FuncRef whose *body* only uses Params (captures become Params too).
        // Creation site becomes: immediately-invoked style is wrong for `let f =`.
        // Practical MVP: copy scalar captures by promoting them to lambda Params and
        // rewriting the stored function to accept only user params while baking
        // capture *values* when the outer binding is a literal; otherwise LL3001.
        let mut subst: IndexMap<String, Expr> = IndexMap::new();
        let mut capture_params: Vec<(String, Ty)> = Vec::new();
        let mut capture_exprs: Vec<Expr> = Vec::new();
        for name in &free {
            let ty = scope.get(name).unwrap_or(Ty::Error);
            if let Some(lit) = scope.const_literal(name) {
                subst.insert(name.clone(), lit);
                continue;
            }
            let ok_copy = matches!(
                ty,
                Ty::Int
                    | Ty::Int8
                    | Ty::Int16
                    | Ty::Int32
                    | Ty::Int64
                    | Ty::UInt
                    | Ty::Bool
                    | Ty::Float
                    | Ty::Double
                    | Ty::Char
                    | Ty::String
                    | Ty::Class(_)
                    | Ty::Struct(_)
            );
            if ok_copy {
                // Phase 1.2: promote capture to a leading parameter; FuncRef carries
                // the creation-site expression so calls prepend it automatically.
                let cap_name = format!("__cap_{name}");
                capture_params.push((cap_name.clone(), ty.clone()));
                capture_exprs.push(Expr::Local(name.clone()));
                subst.insert(name.clone(), Expr::Local(cap_name));
            } else {
                self.error(
                    diag::LL3001,
                    format!(
                        "closure capture of `{name}: {}` is not supported yet",
                        ty.display()
                    ),
                    span,
                );
            }
        }
        if !subst.is_empty() {
            subst_locals(&mut body_e, &subst);
        }

        let ret = self.expr_ty(&body_e, scope);
        let id = self.next_lambda;
        self.next_lambda += 1;
        let name = format!("__lambda_{id}");
        let mangled = if self.namespace.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", self.namespace, name)
        };
        let user_param_tys: Vec<Ty> = sig_params.iter().map(|(_, t)| t.clone()).collect();
        let mut full_params = capture_params;
        full_params.extend(sig_params);
        let sig = FuncSig {
            params: full_params.clone(),
            ret: ret.clone(),
            throws: None,
            is_async: false,
            where_bounds: IndexMap::new(),
        };
        self.module_funcs
            .insert(name.clone(), (sig.clone(), mangled.clone()));
        self.lambda_funcs.push(Function {
            name: mangled.clone(),
            params: full_params,
            ret: ret.clone(),
            throws: None,
            is_method: false,
            self_ty: None,
            is_virtual: false,
            is_override: false,
            is_async: false,
            body: Some(Body {
                stmts: vec![Stmt::Return(Some(body_e))],
            }),
        });
        // Surface type hides capture params (call sites prepend via FuncRef.captures).
        Expr::FuncRef {
            name: mangled,
            ty: Ty::Func {
                params: user_param_tys,
                ret: Box::new(ret),
                throws: None,
            },
            captures: capture_exprs,
        }
    }

    /// Phase 1.2: `[Int].map/filter/any/all/reduce/sorted` via synthesized helpers.
    fn check_int_array_hof(
        &mut self,
        method: &str,
        base_expr: Expr,
        args: &[CallArg],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
        span: Span,
    ) -> Option<Expr> {
        let arr_ty = Ty::Array(Box::new(Ty::Int));
        if method == "sorted" {
            // No closure — runtime sorts a copy.
            return Some(Expr::Call {
                callee: Callee::Builtin("array_sorted_i64".into()),
                args: vec![base_expr],
                ty: arr_ty,
                is_try: TryKind::None,
            });
        }
        if method == "reduce" {
            let id = self.next_lambda as u32;
            self.next_lambda += 1;
            return self.check_int_array_reduce(
                base_expr, args, scope, self_ty, throws, throws_ok, span, id,
            );
        }
        if args.is_empty() {
            self.error(
                diag::LL2000,
                format!("`{method}` expects a closure argument"),
                span,
            );
            return Some(Expr::Nil);
        }
        let pred = self.check_expr(
            &args[0].value,
            scope,
            self_ty.clone(),
            throws.clone(),
            throws_ok,
        );
        let (fn_name, captures) = match &pred {
            Expr::FuncRef {
                name, captures, ..
            } => (name.clone(), captures.clone()),
            _ => {
                self.error(
                    diag::LL2000,
                    format!("`{method}` expects a function/closure argument"),
                    span,
                );
                return Some(Expr::Nil);
            }
        };

        let id = self.next_lambda;
        self.next_lambda += 1;
        let helper = if self.namespace.is_empty() {
            format!("__hof_{method}_{id}")
        } else {
            format!("{}.__hof_{method}_{id}", self.namespace)
        };

        let call_lambda = |x: Expr| -> Expr {
            let mut a = captures.clone();
            a.push(x);
            Expr::Call {
                callee: Callee::Func(fn_name.clone()),
                args: a,
                ty: if method == "map" || method == "reduce" {
                    Ty::Int
                } else {
                    Ty::Bool
                },
                is_try: TryKind::None,
            }
        };

        let (ret_ty, body_stmts) = match method {
            "map" => {
                let stmts = vec![
                    Stmt::Var {
                        name: "out".into(),
                        ty: arr_ty.clone(),
                        init: Some(Expr::ArrayLit {
                            elems: vec![],
                            elem_ty: Ty::Int,
                        }),
                    },
                    Stmt::ForEach {
                        name: "x".into(),
                        array: Expr::Param("arr".into()),
                        body: Body {
                            stmts: vec![Stmt::Expr(Expr::Call {
                                callee: Callee::Builtin("array_append_i64".into()),
                                args: vec![Expr::Local("out".into()), call_lambda(Expr::Local("x".into()))],
                                ty: Ty::Void,
                                is_try: TryKind::None,
                            })],
                        },
                    },
                    Stmt::Return(Some(Expr::Local("out".into()))),
                ];
                (arr_ty.clone(), stmts)
            }
            "filter" => {
                let stmts = vec![
                    Stmt::Var {
                        name: "out".into(),
                        ty: arr_ty.clone(),
                        init: Some(Expr::ArrayLit {
                            elems: vec![],
                            elem_ty: Ty::Int,
                        }),
                    },
                    Stmt::ForEach {
                        name: "x".into(),
                        array: Expr::Param("arr".into()),
                        body: Body {
                            stmts: vec![Stmt::If {
                                cond: call_lambda(Expr::Local("x".into())),
                                then_body: Body {
                                    stmts: vec![Stmt::Expr(Expr::Call {
                                        callee: Callee::Builtin("array_append_i64".into()),
                                        args: vec![
                                            Expr::Local("out".into()),
                                            Expr::Local("x".into()),
                                        ],
                                        ty: Ty::Void,
                                        is_try: TryKind::None,
                                    })],
                                },
                                else_body: None,
                            }],
                        },
                    },
                    Stmt::Return(Some(Expr::Local("out".into()))),
                ];
                (arr_ty.clone(), stmts)
            }
            "any" => {
                let stmts = vec![
                    Stmt::ForEach {
                        name: "x".into(),
                        array: Expr::Param("arr".into()),
                        body: Body {
                            stmts: vec![Stmt::If {
                                cond: call_lambda(Expr::Local("x".into())),
                                then_body: Body {
                                    stmts: vec![Stmt::Return(Some(Expr::Bool(true)))],
                                },
                                else_body: None,
                            }],
                        },
                    },
                    Stmt::Return(Some(Expr::Bool(false))),
                ];
                (Ty::Bool, stmts)
            }
            "all" => {
                let stmts = vec![
                    Stmt::ForEach {
                        name: "x".into(),
                        array: Expr::Param("arr".into()),
                        body: Body {
                            stmts: vec![Stmt::If {
                                cond: Expr::Unary {
                                    op: UnaryOp::Not,
                                    expr: Box::new(call_lambda(Expr::Local("x".into()))),
                                    ty: Ty::Bool,
                                },
                                then_body: Body {
                                    stmts: vec![Stmt::Return(Some(Expr::Bool(false)))],
                                },
                                else_body: None,
                            }],
                        },
                    },
                    Stmt::Return(Some(Expr::Bool(true))),
                ];
                (Ty::Bool, stmts)
            }
            _ => return None,
        };

        let params = vec![("arr".into(), arr_ty.clone())];
        let sig = FuncSig {
            params: params.clone(),
            ret: ret_ty.clone(),
            throws: None,
            is_async: false,
            where_bounds: IndexMap::new(),
        };
        let short = helper.rsplit('.').next().unwrap_or(&helper).to_string();
        self.module_funcs
            .insert(short.clone(), (sig, helper.clone()));
        self.lambda_funcs.push(Function {
            name: helper.clone(),
            params,
            ret: ret_ty.clone(),
            throws: None,
            is_method: false,
            self_ty: None,
            is_virtual: false,
            is_override: false,
            is_async: false,
            body: Some(Body { stmts: body_stmts }),
        });
        Some(Expr::Call {
            callee: Callee::Func(helper),
            args: vec![base_expr],
            ty: ret_ty,
            is_try: TryKind::None,
        })
    }

    fn check_int_array_reduce(
        &mut self,
        base_expr: Expr,
        args: &[CallArg],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
        span: Span,
        id: u32,
    ) -> Option<Expr> {
        let arr_ty = Ty::Array(Box::new(Ty::Int));
        let init_e = self.check_expr(
            &args[0].value,
            scope,
            self_ty.clone(),
            throws.clone(),
            throws_ok,
        );
        let pred = self.check_expr(
            &args[1].value,
            scope,
            self_ty,
            throws,
            throws_ok,
        );
        let (fn_name, captures) = match &pred {
            Expr::FuncRef {
                name, captures, ..
            } => (name.clone(), captures.clone()),
            _ => {
                self.error(
                    diag::LL2000,
                    "`reduce` expects `(acc, x) => …` as the second argument",
                    span,
                );
                return Some(Expr::Nil);
            }
        };
        let helper = if self.namespace.is_empty() {
            format!("__hof_reduce_{id}")
        } else {
            format!("{}.__hof_reduce_{id}", self.namespace)
        };
        let call_lambda = |acc: Expr, x: Expr| -> Expr {
            let mut a = captures.clone();
            a.push(acc);
            a.push(x);
            Expr::Call {
                callee: Callee::Func(fn_name.clone()),
                args: a,
                ty: Ty::Int,
                is_try: TryKind::None,
            }
        };
        let stmts = vec![
            Stmt::Var {
                name: "acc".into(),
                ty: Ty::Int,
                init: Some(Expr::Param("init".into())),
            },
            Stmt::ForEach {
                name: "x".into(),
                array: Expr::Param("arr".into()),
                body: Body {
                    stmts: vec![Stmt::Assign {
                        target: Expr::Local("acc".into()),
                        value: call_lambda(Expr::Local("acc".into()), Expr::Local("x".into())),
                    }],
                },
            },
            Stmt::Return(Some(Expr::Local("acc".into()))),
        ];
        let params = vec![
            ("arr".into(), arr_ty),
            ("init".into(), Ty::Int),
        ];
        let sig = FuncSig {
            params: params.clone(),
            ret: Ty::Int,
            throws: None,
            is_async: false,
            where_bounds: IndexMap::new(),
        };
        let short = helper.rsplit('.').next().unwrap_or(&helper).to_string();
        self.module_funcs
            .insert(short, (sig, helper.clone()));
        self.lambda_funcs.push(Function {
            name: helper.clone(),
            params,
            ret: Ty::Int,
            throws: None,
            is_method: false,
            self_ty: None,
            is_virtual: false,
            is_override: false,
            is_async: false,
            body: Some(Body { stmts }),
        });
        Some(Expr::Call {
            callee: Callee::Func(helper),
            args: vec![base_expr, init_e],
            ty: Ty::Int,
            is_try: TryKind::None,
        })
    }

    /// Typed `Env.*` getters: arity 1 → optional / parse-or-none; arity 2 → defaulted.
    fn check_env_call(
        &mut self,
        method: &str,
        args: &[CallArg],
        scope: &Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
        throws_ok: bool,
        try_kind: TryCallKind,
    ) -> Option<Expr> {
        let n = args.len();
        let call = |this: &mut Self, bid: &str, params: &[Ty], ret: Ty| {
            let checked =
                this.check_args_against(args, params, scope, self_ty.clone(), throws.clone(), throws_ok);
            Expr::Call {
                callee: Callee::Builtin(bid.into()),
                args: checked,
                ty: ret,
                is_try: map_try(try_kind),
            }
        };
        match (method, n) {
            ("get" | "string", 1) => Some(call(self, "env_get", &[Ty::String], Ty::optional(Ty::String))),
            ("string", 2) => Some(call(
                self,
                "env_string_or",
                &[Ty::String, Ty::String],
                Ty::String,
            )),
            ("integer" | "int" | "int64" | "uint", 1) => {
                let ret = match method {
                    "uint" => Ty::optional(Ty::UInt),
                    "int64" => Ty::optional(Ty::Int64),
                    _ => Ty::optional(Ty::Int),
                };
                Some(call(self, "env_get_i64", &[Ty::String], ret))
            }
            ("integer" | "int" | "int64" | "uint", 2) => {
                let (def_ty, ret) = match method {
                    "uint" => (Ty::UInt, Ty::UInt),
                    "int64" => (Ty::Int64, Ty::Int64),
                    _ => (Ty::Int, Ty::Int),
                };
                Some(call(self, "env_get_i64_or", &[Ty::String, def_ty], ret))
            }
            ("bool", 1) => Some(call(
                self,
                "env_get_bool",
                &[Ty::String],
                Ty::optional(Ty::Bool),
            )),
            ("bool", 2) => Some(call(
                self,
                "env_get_bool_or",
                &[Ty::String, Ty::Bool],
                Ty::Bool,
            )),
            ("float" | "double", 1) => Some(call(
                self,
                "env_get_f64",
                &[Ty::String],
                Ty::optional(Ty::Double),
            )),
            ("float" | "double", 2) => Some(call(
                self,
                "env_get_f64_or",
                &[Ty::String, Ty::Double],
                Ty::Double,
            )),
            ("decimal", 1) => Some(call(
                self,
                "env_get_decimal",
                &[Ty::String],
                Ty::optional(Ty::Named("Decimal".into())),
            )),
            ("decimal", 2) => Some(call(
                self,
                "env_get_decimal_or",
                &[Ty::String, Ty::Named("Decimal".into())],
                Ty::Named("Decimal".into()),
            )),
            ("load" | "loadDotEnv", 0) => Some(Expr::Call {
                callee: Callee::Builtin("env_load_dotenv".into()),
                args: vec![Expr::StringLit(".env".into())],
                ty: Ty::Void,
                is_try: TryKind::None,
            }),
            ("load" | "loadDotEnv", 1) => Some(call(
                self,
                "env_load_dotenv",
                &[Ty::String],
                Ty::Void,
            )),
            _ => None,
        }
    }

    fn deprecated_message(attrs: &[lllang_syntax::Attribute]) -> Option<String> {
        for a in attrs {
            if a.name != "deprecated" {
                continue;
            }
            for arg in &a.args {
                if let AExpr::LiteralString { parts, .. } = arg {
                    let mut s = String::new();
                    for p in parts {
                        if let StringPart::Text(t) = p {
                            s.push_str(t);
                        }
                    }
                    return Some(s);
                }
            }
            return Some(String::new());
        }
        None
    }

    fn note_deprecated_use(&mut self, name: &str, span: Span) {
        if let Some(msg) = self.deprecated.get(name).cloned() {
            let detail = if msg.is_empty() {
                format!("`{name}` is deprecated")
            } else {
                format!("`{name}` is deprecated: {msg}")
            };
            self.warn(diag::LL4001, detail, span);
        }
    }

    fn extern_cname_from_attrs(attrs: &[lllang_syntax::Attribute]) -> Option<String> {
        for a in attrs {
            if a.name != "extern" {
                continue;
            }
            for arg in &a.args {
                if let AExpr::LiteralString { parts, .. } = arg {
                    let mut s = String::new();
                    for p in parts {
                        if let StringPart::Text(t) = p {
                            s.push_str(t);
                        }
                    }
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
        None
    }

    fn collect_generic_constraints(&self, f: &FuncDecl) -> IndexMap<String, Vec<String>> {
        let mut out: IndexMap<String, Vec<String>> = IndexMap::new();
        for g in &f.generics {
            let mut bounds = Vec::new();
            if let Some(ref c) = g.constraint {
                if let Some(n) = type_name_from_ast(c) {
                    bounds.push(n);
                }
            }
            out.insert(g.name.clone(), bounds);
        }
        for w in &f.where_clause {
            let Some(subject) = type_name_from_ast(&w.subject) else {
                continue;
            };
            let bound = match &w.bound {
                GenericBound::Type(t) | GenericBound::SameType(t) => type_name_from_ast(t),
            };
            if let Some(b) = bound {
                out.entry(subject).or_default().push(b);
            }
        }
        out
    }

    fn type_conforms_to(&self, ty: &Ty, protocol: &str) -> bool {
        // Builtin / value types are Sendable.
        if protocol == "Sendable" {
            match ty {
                Ty::Bool
                | Ty::Int
                | Ty::Int8
                | Ty::Int16
                | Ty::Int32
                | Ty::Int64
                | Ty::UInt
                | Ty::Float
                | Ty::Double
                | Ty::Char
                | Ty::String
                | Ty::Struct(_)
                | Ty::Enum(_) => {
                    return true;
                }
                Ty::Named(n)
                    if matches!(
                        n.as_str(),
                        "UInt8"
                            | "UInt16"
                            | "UInt32"
                            | "UInt64"
                            | "Float32"
                            | "Float64"
                            | "Decimal"
                    ) =>
                {
                    return true;
                }
                Ty::Optional(inner) => return self.type_conforms_to(inner, protocol),
                _ => {}
            }
        }
        let Some(name) = type_name_key(ty) else {
            return false;
        };
        let mut cur = Some(name.to_string());
        while let Some(n) = cur {
            let Some(entry) = self.types.get(&n) else {
                break;
            };
            if entry.conforms.iter().any(|c| c == protocol) {
                return true;
            }
            cur = entry.parent.clone();
        }
        false
    }

    /// Assignability including protocol existential conformance.
    fn ty_assignable(&self, expected: &Ty, found: &Ty) -> bool {
        if expected.assignable_from(found) {
            return true;
        }
        match expected {
            Ty::Protocol(p) => self.type_conforms_to(found, p) || matches!(found, Ty::Protocol(q) if q == p),
            Ty::Optional(inner) => {
                if let Ty::Protocol(p) = inner.as_ref() {
                    self.type_conforms_to(found, p)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Pointer-sized results that `ll_task_spawn_ret` / `ll_task_join_ret` can carry.
    fn ty_is_task_ret_ptr(ty: &Ty) -> bool {
        matches!(
            ty,
            Ty::String
                | Ty::Struct(_)
                | Ty::Class(_)
                | Ty::Named(_)
                | Ty::Enum(_)
                | Ty::Optional(_)
                | Ty::Protocol(_)
        )
    }

    fn func_is_async(&self, mangled: &str) -> bool {
        if self
            .hir_funcs
            .iter()
            .any(|f| f.name == mangled && f.is_async)
        {
            return true;
        }
        self.module_funcs
            .values()
            .any(|(sig, m)| m == mangled && sig.is_async)
    }

    /// Rewrite `await` of Task / zero-arg async call into spawn+join; reject pure no-ops.
    fn lower_await(&mut self, inner: Expr, span: Span) -> Expr {
        // `await expr catch …` — lower the awaited subexpression, keep the catch wrapper.
        if let Expr::Catch {
            expr,
            binder,
            fallback,
            ty,
        } = inner
        {
            let awaited = self.lower_await(*expr, span);
            return Expr::Catch {
                expr: Box::new(awaited),
                binder,
                fallback,
                ty,
            };
        }
        let ty = type_of_expr(&inner);
        if matches!(&ty, Ty::Named(n) if n == "Task") {
            return Expr::Call {
                callee: Callee::Builtin("task_join".into()),
                args: vec![inner],
                ty: Ty::Void,
                is_try: TryKind::None,
            };
        }
        match &inner {
            Expr::Call {
                callee: Callee::Func(name),
                args,
                ty: ret,
                is_try,
            } if args.is_empty() && self.func_is_async(name) => {
                let returns_value = Self::ty_is_task_ret_ptr(ret);
                if matches!(ret, Ty::Void) || returns_value {
                    let spawn = Expr::Call {
                        callee: Callee::TaskSpawn {
                            name: name.clone(),
                            returns_value,
                        },
                        args: vec![],
                        ty: Ty::Named("Task".into()),
                        is_try: TryKind::None,
                    };
                    return if returns_value {
                        Expr::Call {
                            callee: Callee::Builtin("task_join_ret".into()),
                            args: vec![spawn],
                            ty: ret.clone(),
                            is_try: *is_try,
                        }
                    } else {
                        Expr::Call {
                            callee: Callee::Builtin("task_join".into()),
                            args: vec![spawn],
                            ty: Ty::Void,
                            is_try: TryKind::None,
                        }
                    };
                }
                // Scalar-returning async: runs on the caller until spawn_ret covers i64.
                return inner;
            }
            Expr::Call {
                callee: Callee::Func(name),
                ..
            } if self.func_is_async(name) => {
                // Async with arguments — blocking MVP (Http-style).
                return inner;
            }
            Expr::Call {
                callee: Callee::Builtin(_),
                ..
            } => {
                // Blocking async builtins (Http, SQL, …).
                return inner;
            }
            _ => {}
        }
        self.error(
            diag::LL3001,
            "`await` requires a `Task`, `async let` binding, or async function call",
            span,
        );
        inner
    }

    fn check_async_let(
        &mut self,
        b: &PatternBinding,
        scope: &mut Scope,
        self_ty: Option<Ty>,
        throws: Option<Ty>,
    ) -> Vec<Stmt> {
        if !self.in_async {
            self.error(
                diag::LL3001,
                "`async let` is only allowed inside an `async` function",
                b.span,
            );
        }
        let (name, init_expr, _ann) = self.binding_info(b, scope, self_ty.clone(), throws.clone());
        let Some(init) = init_expr else {
            self.error(
                diag::LL3001,
                "`async let` requires an initializer",
                b.span,
            );
            return vec![];
        };
        match init {
            Expr::Call {
                callee: Callee::Func(fname),
                args,
                ty: ret,
                ..
            } if args.is_empty() && self.func_is_async(&fname) => {
                let returns_value = Self::ty_is_task_ret_ptr(&ret);
                if !(matches!(ret, Ty::Void) || returns_value) {
                    self.error(
                        diag::LL3001,
                        "`async let` MVP supports void or pointer-returning zero-arg async calls",
                        b.span,
                    );
                    return vec![];
                }
                let result_ty = ret.clone();
                let spawn = Expr::Call {
                    callee: Callee::TaskSpawn {
                        name: fname,
                        returns_value,
                    },
                    args: vec![],
                    ty: Ty::Named("Task".into()),
                    is_try: TryKind::None,
                };
                self.async_lets.insert(name.clone(), result_ty.clone());
                scope.insert(name.clone(), Ty::Named("Task".into()));
                self.index.define(
                    &name,
                    &self.current_file,
                    b.span,
                    Some(&result_ty),
                    SymbolKind::Local,
                );
                self.index.inlay(
                    &self.current_file,
                    b.span,
                    format!(": {} (async let)", result_ty.display()),
                );
                vec![Stmt::Let {
                    name,
                    ty: Ty::Named("Task".into()),
                    init: spawn,
                    async_let_result: Some(result_ty),
                }]
            }
            _ => {
                self.error(
                    diag::LL3001,
                    "`async let` MVP requires a zero-argument `async` function call",
                    b.span,
                );
                vec![]
            }
        }
    }

    /// Cancel + join any `async let` not yet awaited (best-effort early-exit cleanup).
    fn async_let_cleanup_stmts(&self) -> Vec<Stmt> {
        let mut out = vec![];
        for (name, result_ty) in &self.async_lets {
            if self.async_lets_joined.contains_key(name) {
                continue;
            }
            let handle = Expr::Local(name.clone());
            out.push(Stmt::Expr(Expr::Call {
                callee: Callee::Builtin("task_cancel".into()),
                args: vec![handle.clone()],
                ty: Ty::Void,
                is_try: TryKind::None,
            }));
            if Self::ty_is_task_ret_ptr(result_ty) {
                out.push(Stmt::Expr(Expr::Call {
                    callee: Callee::Builtin("task_join_ret".into()),
                    args: vec![handle],
                    ty: result_ty.clone(),
                    is_try: TryKind::None,
                }));
            } else {
                out.push(Stmt::Expr(Expr::Call {
                    callee: Callee::Builtin("task_join".into()),
                    args: vec![handle],
                    ty: Ty::Void,
                    is_try: TryKind::None,
                }));
            }
        }
        out
    }

    fn is_sendable(&self, ty: &Ty) -> bool {
        self.type_conforms_to(ty, "Sendable")
            || matches!(
                ty,
                Ty::Struct(_)
                    | Ty::Enum(_)
                    | Ty::Bool
                    | Ty::Int
                    | Ty::Int8
                    | Ty::Int16
                    | Ty::Int32
                    | Ty::Int64
                    | Ty::UInt
                    | Ty::Float
                    | Ty::Double
                    | Ty::Char
                    | Ty::String
            )
            || matches!(ty, Ty::Class(_)) && {
                // Classes need explicit `: Sendable` (actors get it automatically).
                type_name_key(ty)
                    .and_then(|n| self.types.get(n))
                    .map(|e| e.conforms.iter().any(|c| c == "Sendable") || e.is_actor)
                    .unwrap_or(false)
            }
            || matches!(ty, Ty::Named(n) if n == "CancellationToken" || n == "Task" || n == "Channel" || n == "WaitGroup" || n == "TaskGroup")
    }

    /// Resolve `(String) -> String` for `HttpServer.serve`.
    fn resolve_string_to_string_func(&self, expr: &Expr, scope: &Scope) -> Option<String> {
        let ok_sig = |params: &[Ty], ret: &Ty| -> bool {
            params.len() == 1
                && matches!(params.first(), Some(Ty::String))
                && matches!(ret, Ty::String)
        };
        let lookup = |name: &str| -> Option<String> {
            if let Some((sig, mangled)) = self.module_funcs.get(name) {
                let pts: Vec<Ty> = sig.params.iter().map(|(_, t)| t.clone()).collect();
                if ok_sig(&pts, &sig.ret) {
                    return Some(mangled.clone());
                }
            }
            if let Some((mangled, captures)) = scope.func_ref(name) {
                if captures.is_empty() {
                    if let Some(Ty::Func { params, ret, .. }) = scope.get(name) {
                        if ok_sig(&params, ret.as_ref()) {
                            return Some(mangled.to_string());
                        }
                    }
                }
            }
            // Bare name may be fully qualified in module_funcs values.
            for (_, (sig, mangled)) in &self.module_funcs {
                if mangled == name || mangled.ends_with(&format!(".{name}")) {
                    let pts: Vec<Ty> = sig.params.iter().map(|(_, t)| t.clone()).collect();
                    if ok_sig(&pts, &sig.ret) {
                        return Some(mangled.clone());
                    }
                }
            }
            None
        };
        match expr {
            Expr::Local(name) | Expr::Param(name) => lookup(name),
            Expr::FuncRef {
                name,
                ty,
                captures,
            } if captures.is_empty() => {
                if let Ty::Func { params, ret, .. } = ty {
                    if ok_sig(params, ret.as_ref()) {
                        return Some(name.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Resolve a zero-arg `-> Void` free function for `TaskGroup.add` / `@run`-style APIs.
    fn resolve_void_zero_arg_func(&self, expr: &Expr, scope: &Scope) -> Option<String> {
        let lookup = |name: &str| -> Option<String> {
            if let Some((sig, mangled)) = self.module_funcs.get(name) {
                if sig.params.is_empty() && matches!(sig.ret, Ty::Void) {
                    return Some(mangled.clone());
                }
            }
            // `let f = worker` / scope-bound FuncRef.
            if let Some((mangled, captures)) = scope.func_ref(name) {
                if captures.is_empty() {
                    if let Some(Ty::Func { params, ret, .. }) = scope.get(name) {
                        if params.is_empty() && matches!(ret.as_ref(), Ty::Void) {
                            return Some(mangled.to_string());
                        }
                    }
                }
            }
            None
        };
        match expr {
            Expr::Local(name) | Expr::Param(name) => lookup(name),
            Expr::FuncRef {
                name,
                ty,
                captures,
            } if captures.is_empty() => {
                if let Ty::Func { params, ret, .. } = ty {
                    if params.is_empty() && matches!(ret.as_ref(), Ty::Void) {
                        return Some(name.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Non-Sendable values must not cross into an actor method call.
    fn check_actor_isolation(
        &mut self,
        receiver_ty: &Ty,
        args: &[Expr],
        span: Span,
        scope: &Scope,
    ) {
        let Some(name) = type_name_key(receiver_ty) else {
            return;
        };
        let Some(entry) = self.types.get(name) else {
            return;
        };
        if !entry.is_actor {
            return;
        }
        for a in args {
            let ty = self.expr_ty(a, scope);
            if !self.is_sendable(&ty) && !ty.is_error() {
                self.error(
                    diag::LL2005,
                    format!(
                        "argument of type `{}` is not Sendable and cannot cross actor isolation",
                        ty.display()
                    ),
                    span,
                );
            }
        }
    }

    fn concrete_type_label(ty: &Ty) -> String {
        match ty {
            Ty::Struct(n) | Ty::Class(n) | Ty::Enum(n) | Ty::Named(n) | Ty::Protocol(n) => {
                n.clone()
            }
            Ty::Int => "Int".into(),
            Ty::String => "String".into(),
            Ty::Bool => "Bool".into(),
            Ty::Double => "Double".into(),
            Ty::Optional(inner) => format!("Opt{}", Self::concrete_type_label(inner)),
            _ => "T".into(),
        }
    }

    /// Monomorphize a free generic function for the given argument types.
    fn specialize_generic_call(
        &mut self,
        simple_name: &str,
        arg_tys: &[Ty],
        span: Span,
    ) -> Option<(String, FuncSig)> {
        let tmpl = self.generic_templates.get(simple_name)?.clone();
        let type_params: Vec<String> = tmpl.decl.generics.iter().map(|g| g.name.clone()).collect();
        let mut subst: IndexMap<String, Ty> = IndexMap::new();
        self.active_type_params = type_params.clone();
        let (params, _) = self.func_params(&tmpl.decl, None);
        self.active_type_params.clear();
        for ((_, pty), aty) in params.iter().zip(arg_tys.iter()) {
            Self::infer_type_params(pty, aty, &type_params, &mut subst);
        }
        for tp in &type_params {
            if !subst.contains_key(tp) {
                self.error(
                    diag::LL2002,
                    format!("cannot infer type parameter `{tp}` for `{simple_name}`"),
                    span,
                );
                return None;
            }
        }
        for (tp, bounds) in &tmpl.constraints {
            let Some(concrete) = subst.get(tp) else {
                continue;
            };
            for proto in bounds {
                // Skip same-type / class bounds that aren't protocols.
                if !self.types.get(proto).map(|e| matches!(e.kind, TypeKind::Protocol)).unwrap_or(false)
                    && !prelude::prelude_protocols().contains(&proto.as_str())
                {
                    continue;
                }
                if !self.type_conforms_to(concrete, proto) {
                    self.error(
                        diag::LL2002,
                        format!(
                            "type `{}` does not conform to protocol `{proto}` (required by `{simple_name}`)",
                            concrete.display()
                        ),
                        span,
                    );
                    return None;
                }
            }
        }
        let suffix: Vec<_> = type_params
            .iter()
            .map(|tp| Self::concrete_type_label(subst.get(tp).unwrap()))
            .collect();
        let mangled = if !tmpl.namespace.is_empty() {
            format!("{}.{}_{}", tmpl.namespace, simple_name, suffix.join("_"))
        } else {
            format!("{}_{}", simple_name, suffix.join("_"))
        };
        if let Some(sig) = self.specialized_funcs.get(&mangled) {
            return Some((mangled, sig.clone()));
        }

        self.current_file = tmpl.file.clone();
        self.namespace = tmpl.namespace.clone();
        self.type_param_subst = subst.clone();
        let (spec_params, _) = self.func_params(&tmpl.decl, None);
        let ret = tmpl
            .decl
            .return_type
            .as_ref()
            .map(|t| self.resolve_ast_type(t))
            .unwrap_or(Ty::Void);
        let throws = throws_type(&tmpl.decl.throws);
        let mut scope = Scope::new();
        for (n, t) in &spec_params {
            scope.insert_param(n.clone(), t.clone());
        }
        let body = self.check_func_body(&tmpl.decl, &scope, None, throws.clone());
        self.type_param_subst.clear();

        let sig = FuncSig {
     params: spec_params.clone(),
     ret: ret.clone(),
     throws: throws.clone(),
     is_async: false,
                    where_bounds: IndexMap::new(),
                };
        self.hir_funcs.push(Function {
            name: mangled.clone(),
            params: spec_params,
            ret,
            throws,
            is_method: false,
            self_ty: None,
            is_virtual: false,
            is_override: false,
            is_async: false,
            body: Some(body),
        });
        self.specialized_funcs.insert(mangled.clone(), sig.clone());
        Some((mangled, sig))
    }

    fn infer_type_params(
        pattern: &Ty,
        concrete: &Ty,
        type_params: &[String],
        subst: &mut IndexMap<String, Ty>,
    ) {
        match (pattern, concrete) {
            (Ty::Named(n), c) if type_params.iter().any(|p| p == n) => {
                subst.entry(n.clone()).or_insert_with(|| c.clone());
            }
            (Ty::Optional(p), Ty::Optional(c)) => {
                Self::infer_type_params(p, c, type_params, subst);
            }
            (Ty::Array(p), Ty::Array(c)) => {
                Self::infer_type_params(p, c, type_params, subst);
            }
            _ => {}
        }
    }

    /// Lift non-optional values into `T?` and specialize `nil` to `OptionalNone(T)`.
    fn coerce_to(&mut self, expr: Expr, expected: &Ty, scope: &Scope, span: Span) -> Expr {
        let got = self.expr_ty(&expr, scope);
        if expected == &got {
            return expr;
        }
        match expected {
            Ty::Optional(inner) => {
                if matches!(expr, Expr::Nil) {
                    return Expr::OptionalNone((**inner).clone());
                }
                if got == *inner.as_ref()
                    || (!matches!(got, Ty::Optional(_)) && expected.assignable_from(&got))
                {
                    // Already optional with matching inner, or lift T → T?.
                    if matches!(got, Ty::Optional(_)) {
                        return expr;
                    }
                    return Expr::OptionalSome(Box::new(expr));
                }
                if matches!(got, Ty::Optional(_)) {
                    return expr;
                }
                let _ = span;
                expr
            }
            _ => expr,
        }
    }

    /// `opt?.field` — short-circuit to `.none` when `opt` is none.
    fn check_optional_member(
        &mut self,
        base: Expr,
        name: &str,
        scope: &Scope,
        self_ty: Option<Ty>,
    ) -> Expr {
        self.wrap_optional_chain(base, scope, |this, unwrapped, scope| {
            let member = this.check_member(unwrapped, name, scope, self_ty.clone());
            let ty = this.expr_ty(&member, scope);
            (member, ty)
        })
    }

    fn wrap_optional_chain<F>(&mut self, base: Expr, scope: &Scope, f: F) -> Expr
    where
        F: FnOnce(&mut Self, Expr, &Scope) -> (Expr, Ty),
    {
        let bty = self.expr_ty(&base, scope);
        let inner_ty = match &bty {
            Ty::Optional(t) => (**t).clone(),
            _ => {
                self.error(
                    diag::LL2002,
                    "optional chaining requires an Optional value",
                    Span::EMPTY,
                );
                return f(self, base, scope).0;
            }
        };
        let unwrapped = Expr::Call {
            callee: Callee::Builtin("unwrap_optional".into()),
            args: vec![base.clone()],
            ty: inner_ty,
            is_try: TryKind::None,
        };
        let (inner_expr, ret_ty) = f(self, unwrapped, scope);
        let opt_ty = Ty::Optional(Box::new(ret_ty.clone()));
        Expr::IfExpr {
            cond: Box::new(Expr::Call {
                callee: Callee::Builtin("optional_is_some".into()),
                args: vec![base],
                ty: Ty::Bool,
                is_try: TryKind::None,
            }),
            then_e: Box::new(Expr::OptionalSome(Box::new(inner_expr))),
            else_e: Box::new(Expr::OptionalNone(ret_ty)),
            ty: opt_ty,
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
        // Enum case without payload: `Color.red`
        if let Expr::Local(ref type_name) = base {
            if let Some(entry) = self.types.get(type_name) {
                if matches!(entry.kind, TypeKind::Enum) {
                    if let Some(case_info) = entry.enum_cases.get(name) {
                        if case_info.fields.is_empty() {
                            return Expr::EnumCase {
                                enum_name: type_name.clone(),
                                case: name.to_string(),
                                args: vec![],
                                ty: Ty::Enum(type_name.clone()),
                            };
                        }
                    }
                }
            }
            // Static type property / zero-arg method: `Args.count`, `Instant.now` (as member)
            if let Some((params, ret, _, bid)) =
                builtin_type_methods().get(&(type_name.clone(), name.to_string()))
            {
                if params.is_empty() {
                    return Expr::Call {
                        callee: Callee::Builtin(bid.to_string()),
                        args: vec![],
                        ty: ret.clone(),
                        is_try: TryKind::None,
                    };
                }
            }
        }
        let base_ty = self.expr_ty(&base, scope);
        if matches!(base_ty, Ty::Array(_)) && name == "count" {
            return Expr::Call {
                callee: Callee::Builtin("array_count".into()),
                args: vec![base],
                ty: Ty::Int,
                is_try: TryKind::None,
            };
        }
        if let Ty::Named(n) = &base_ty {
            if let Some((ty, is_prop, bid)) = builtin_members().get(&(n.clone(), name.to_string())) {
                if *is_prop {
                    return Expr::Call {
                        callee: Callee::Builtin(bid.to_string()),
                        args: vec![base],
                        ty: ty.clone(),
                        is_try: TryKind::None,
                    };
                }
            }
        }
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
                if let Some(get_name) = entry.computed_getters.get(name).cloned() {
                    let (sig, _) = self.lookup_method(&base_ty, &get_name).unwrap_or((
                        FuncSig {
                            params: vec![],
                            ret: Ty::Error,
                            throws: None,
                            is_async: false,
                            where_bounds: IndexMap::new(),
                        },
                        false,
                    ));
                    return Expr::Call {
                        callee: Callee::Method {
                            receiver: Box::new(base),
                            name: get_name,
                        },
                        args: vec![],
                        ty: sig.ret,
                        is_try: TryKind::None,
                    };
                }
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

    /// Resolve a `where` subject (`Element`, `Self`, type name) against the call receiver.
    fn where_subject_ty(&self, subject: &str, receiver: &Ty) -> Ty {
        match subject {
            "Element" => match receiver {
                Ty::Array(e) => (**e).clone(),
                _ => Ty::Error,
            },
            "Key" | "Value" => Ty::String, // Dictionary MVP is [String:String]
            "Self" => receiver.clone(),
            other => {
                if type_name_key(receiver) == Some(other) {
                    receiver.clone()
                } else {
                    Ty::Named(other.to_string())
                }
            }
        }
    }

    fn extension_where_ok(&self, receiver: &Ty, bounds: &IndexMap<String, Vec<String>>) -> bool {
        if bounds.is_empty() {
            return true;
        }
        for (subject, protocols) in bounds {
            let subj_ty = self.where_subject_ty(subject, receiver);
            for p in protocols {
                if !self.type_conforms_to(&subj_ty, p) {
                    // Numeric / String builtins satisfy Comparable / Equatable / Hashable.
                    let builtin_ok = matches!(
                        (&subj_ty, p.as_str()),
                        (
                            Ty::Int
                                | Ty::Int8
                                | Ty::Int16
                                | Ty::Int32
                                | Ty::Int64
                                | Ty::UInt
                                | Ty::Float
                                | Ty::Double
                                | Ty::String
                                | Ty::Bool
                                | Ty::Char,
                            "Comparable" | "Equatable" | "Hashable" | "Sendable"
                        )
                    );
                    if !builtin_ok {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn lookup_method(&self, receiver: &Ty, name: &str) -> Option<(FuncSig, bool)> {
        let type_name = type_name_key(receiver)?;
        if let Some(entry) = self.types.get(type_name) {
            // Existential / protocol-typed receiver: always virtual dispatch via witness/vtable.
            if matches!(entry.kind, TypeKind::Protocol) {
                if let Some(m) = entry.methods.get(name) {
                    if self.extension_where_ok(receiver, &m.where_bounds) {
                        return Some((m.clone(), true));
                    }
                }
                if let Some((params, ret, _)) = entry.protocol_methods.get(name) {
                    return Some((
                        FuncSig {
                            params: params.clone(),
                            ret: ret.clone(),
                            throws: None,
                            is_async: false,
                            where_bounds: IndexMap::new(),
                        },
                        true,
                    ));
                }
            }
            if let Some(m) = entry.methods.get(name) {
                if !self.extension_where_ok(receiver, &m.where_bounds) {
                    return None;
                }
                let virt = matches!(entry.kind, TypeKind::Class) && entry.virtual_methods.contains(&name.to_string());
                return Some((m.clone(), virt));
            }
            for proto in &entry.conforms {
                if let Some(p) = self.types.get(proto) {
                    if let Some(m) = p.methods.get(name) {
                        return Some((m.clone(), false));
                    }
                    if let Some((params, ret, _)) = p.protocol_methods.get(name) {
                        return Some((
                            FuncSig {
                                params: params.clone(),
                                ret: ret.clone(),
                                throws: None,
                                is_async: false,
                    where_bounds: IndexMap::new(),
                },
                            false,
                        ));
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
                let field_tys: Vec<Ty> = if let Ty::Enum(ename) = subject_ty {
                    self.types
                        .get(ename)
                        .and_then(|e| e.enum_cases.get(case))
                        .map(|c| c.fields.iter().map(|(_, t)| t.clone()).collect())
                        .unwrap_or_default()
                } else {
                    vec![]
                };
                let mut binding_names = Vec::new();
                for (i, p) in subpatterns.iter().enumerate() {
                    if let APat::Ident { name, .. } = p {
                        let fty = field_tys.get(i).cloned().unwrap_or(Ty::Int);
                        binds.insert(name.clone(), fty);
                        binding_names.push(name.clone());
                    }
                }
                (
                    Pattern::EnumCase {
                        case: case.clone(),
                        bindings: binding_names,
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
        if catches.is_empty() {
            return inner;
        }
        // Nest last → first so the first clause is tried first at runtime.
        let default_err = throws.clone().unwrap_or(Ty::Named("Error".into()));
        let mut nested = inner;
        for clause in catches.iter().rev() {
            let (binder, bind_ty) = clause
                .pattern
                .as_ref()
                .map(|p| match p {
                    APat::Ident { name, .. } => (name.clone(), default_err.clone()),
                    APat::Typed { pattern, ty, .. } => {
                        let t = self.resolve_ast_type(ty);
                        let n = match pattern.as_ref() {
                            APat::Ident { name, .. } => name.clone(),
                            _ => "e".into(),
                        };
                        (n, t)
                    }
                    _ => ("e".into(), default_err.clone()),
                })
                .unwrap_or_else(|| ("e".into(), default_err.clone()));
            let mut arm_scope = scope.clone();
            arm_scope.insert(binder.clone(), bind_ty);
            let fallback = match &clause.body {
                CatchBody::Arrow(e) => {
                    self.check_expr(e, &arm_scope, self_ty.clone(), throws.clone(), throws_ok)
                }
                CatchBody::Block(b) => {
                    let stmts = self.check_block_stmts(
                        &b.stmts,
                        &mut arm_scope,
                        self_ty.clone(),
                        throws.clone(),
                    );
                    stmts
                        .into_iter()
                        .find_map(|s| match s {
                            Stmt::Expr(e) | Stmt::Return(Some(e)) => Some(e),
                            _ => None,
                        })
                        .unwrap_or(Expr::Int(0))
                }
            };
            let result_ty = type_of_expr(&fallback);
            nested = Expr::Catch {
                expr: Box::new(nested),
                binder,
                fallback: Box::new(fallback),
                ty: result_ty,
            };
        }
        nested
    }

    fn resolve_ast_type(&mut self, ty: &ATy) -> Ty {
        match ty {
            ATy::Named { name, generics, span } => {
                if let Some(subst) = self.type_param_subst.get(name) {
                    return subst.clone();
                }
                if self.active_type_params.iter().any(|p| p == name) {
                    return Ty::Named(name.clone());
                }
                // `Result<T, E>` — typed payloads (Phase 1.1); still one runtime enum shape.
                if name == "Result" && generics.len() == 2 {
                    let t = self.resolve_ast_type(&generics[0]);
                    let e = self.resolve_ast_type(&generics[1]);
                    return Ty::Result {
                        ok: Box::new(t),
                        err: Box::new(e),
                    };
                }
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
                    "UInt8" | "UInt16" | "UInt32" | "UInt64" => {
                        return Ty::Named(name.clone());
                    }
                    "Float" | "Float32" => return Ty::Float,
                    "Float64" => return Ty::Double,
                    "Double" => return Ty::Double,
                    "Decimal" => return Ty::Named("Decimal".into()),
                    "Char" => return Ty::Char,
                    "String" => return Ty::String,
                    "Dictionary" => return Ty::Named("Dictionary".into()),
                    "Set" => return Ty::Named("Set".into()),
                    _ => {}
                };
                if self.types.contains_key(name) {
                    let entry = &self.types[name];
                    return match entry.kind {
                        TypeKind::Struct => Ty::Struct(name.clone()),
                        TypeKind::Class => Ty::Class(name.clone()),
                        TypeKind::Enum => Ty::Enum(name.clone()),
                        TypeKind::Protocol => {
                            // Existential protocol values are Phase 10; allow Ty::Protocol for
                            // internal self-types in protocol default methods only.
                            Ty::Protocol(name.clone())
                        }
                    };
                }
                if prelude::prelude_protocols().contains(&name.as_str()) {
                    return Ty::Protocol(name.clone());
                }
                if !generics.is_empty() && name != "Result" {
                    // Legacy: unknown `Foo<T>` was treated as Optional — keep for non-Result.
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
            ATy::Dictionary { key, value, span } => {
                let k = self.resolve_ast_type(key);
                let v = self.resolve_ast_type(value);
                if k != Ty::String || v != Ty::String {
                    self.error(
                        diag::LL3001,
                        "only `[String: String]` dictionaries are implemented",
                        *span,
                    );
                }
                Ty::Named("Dictionary".into())
            }
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
    /// Immutable bindings initialized to a literal — safe to copy into closures.
    literals: IndexMap<String, Expr>,
    /// Locals bound to a `FuncRef` (mangled name + capture exprs for `let f = …; f(1)`).
    func_refs: IndexMap<String, (String, Vec<Expr>)>,
}

impl Scope {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, name: String, ty: Ty) {
        self.literals.swap_remove(&name);
        self.func_refs.swap_remove(&name);
        self.vars.insert(name, ty);
    }

    fn insert_literal(&mut self, name: String, ty: Ty, lit: Expr) {
        self.func_refs.swap_remove(&name);
        self.vars.insert(name.clone(), ty);
        self.literals.insert(name, lit);
    }

    fn insert_func_ref(&mut self, name: String, ty: Ty, mangled: String, captures: Vec<Expr>) {
        self.literals.swap_remove(&name);
        self.vars.insert(name.clone(), ty);
        self.func_refs.insert(name, (mangled, captures));
    }

    fn insert_param(&mut self, name: String, ty: Ty) {
        self.params.insert(name.clone(), ());
        self.vars.insert(name, ty);
    }

    fn get(&self, name: &str) -> Option<Ty> {
        self.vars.get(name).cloned()
    }

    fn const_literal(&self, name: &str) -> Option<Expr> {
        self.literals.get(name).cloned()
    }

    fn func_ref(&self, name: &str) -> Option<(&str, &[Expr])> {
        self.func_refs
            .get(name)
            .map(|(m, c)| (m.as_str(), c.as_slice()))
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
        Ty::String => Some("String"),
        Ty::Int | Ty::Int64 => Some("Int"),
        Ty::Double => Some("Double"),
        Ty::Float => Some("Float"),
        Ty::Bool => Some("Bool"),
        Ty::Array(_) => Some("Array"),
        _ => None,
    }
}

fn numeric_cast_ok(from: &Ty, to: &Ty) -> bool {
    let num = |t: &Ty| {
        matches!(
            t,
            Ty::Int
                | Ty::Int8
                | Ty::Int16
                | Ty::Int32
                | Ty::Int64
                | Ty::UInt
                | Ty::Float
                | Ty::Double
                | Ty::Bool
                | Ty::Named(_)
        )
    };
    num(from) && num(to)
}

fn type_of_expr(e: &Expr) -> Ty {
    match e {
        Expr::Int(_) => Ty::Int,
        Expr::Float(_) => Ty::Double,
        Expr::Bool(_) => Ty::Bool,
        Expr::Char(_) => Ty::Char,
        Expr::Nil => Ty::Optional(Box::new(Ty::Error)),
        Expr::StringLit(_) | Expr::StringInterp(_) => Ty::String,
        Expr::Local(_) | Expr::Param(_) | Expr::Self_ | Expr::Super => Ty::Error,
        Expr::Call { ty, .. } => ty.clone(),
        Expr::Member { ty, .. } => ty.clone(),
        Expr::Unary { ty, .. } => ty.clone(),
        Expr::Binary { ty, .. } => ty.clone(),
        Expr::IfExpr { ty, .. } => ty.clone(),
        Expr::Switch { ty, .. } => ty.clone(),
        Expr::OptionalSome(inner) => Ty::Optional(Box::new(type_of_expr(inner))),
        Expr::OptionalNone(inner) => Ty::Optional(Box::new(inner.clone())),
        Expr::TryOptional(inner) => Ty::Optional(Box::new(type_of_expr(inner))),
        Expr::Catch { ty, .. } => ty.clone(),
        Expr::Construct { ty, .. } => ty.clone(),
        Expr::EnumCase { ty, .. } => ty.clone(),
        Expr::Cast { to, .. } => to.clone(),
        Expr::ArrayLit { elem_ty, .. } => Ty::Array(Box::new(elem_ty.clone())),
        Expr::DictLit { .. } => Ty::Named("Dictionary".into()),
        Expr::Index { ty, .. } => ty.clone(),
        Expr::FuncRef { ty, .. } => ty.clone(),
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

/// Binding name from `x`, `x: T`, or `let/var x` patterns.
fn pattern_ident_name(pat: &APat) -> Option<&str> {
    match pat {
        APat::Ident { name, .. } => Some(name.as_str()),
        APat::Typed { pattern, .. } | APat::Binding { pattern, .. } => {
            pattern_ident_name(pattern)
        }
        _ => None,
    }
}

fn expr_is_capture_literal(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Int(_)
            | Expr::Float(_)
            | Expr::Bool(_)
            | Expr::Char(_)
            | Expr::StringLit(_)
            | Expr::Nil
    )
}

fn collect_free_locals(
    e: &Expr,
    params: &std::collections::HashSet<String>,
    out: &mut std::collections::BTreeSet<String>,
) {
    match e {
        Expr::Local(n) | Expr::Param(n) => {
            if !params.contains(n) {
                out.insert(n.clone());
            }
        }
        Expr::Unary { expr, .. }
        | Expr::OptionalSome(expr)
        | Expr::TryOptional(expr)
        | Expr::Cast { expr, .. } => collect_free_locals(expr, params, out),
        Expr::Binary { left, right, .. } => {
            collect_free_locals(left, params, out);
            collect_free_locals(right, params, out);
        }
        Expr::Call { callee, args, .. } => {
            if let Callee::Method { receiver, .. } = callee {
                collect_free_locals(receiver, params, out);
            }
            for a in args {
                collect_free_locals(a, params, out);
            }
        }
        Expr::Member { base, .. } => collect_free_locals(base, params, out),
        Expr::Index { base, index, .. } => {
            collect_free_locals(base, params, out);
            collect_free_locals(index, params, out);
        }
        Expr::IfExpr {
            cond,
            then_e,
            else_e,
            ..
        } => {
            collect_free_locals(cond, params, out);
            collect_free_locals(then_e, params, out);
            collect_free_locals(else_e, params, out);
        }
        Expr::ArrayLit { elems, .. } => {
            for el in elems {
                collect_free_locals(el, params, out);
            }
        }
        Expr::DictLit { entries } => {
            for (k, v) in entries {
                collect_free_locals(k, params, out);
                collect_free_locals(v, params, out);
            }
        }
        Expr::Construct { args, .. } => {
            for (_, a) in args {
                collect_free_locals(a, params, out);
            }
        }
        Expr::EnumCase { args, .. } => {
            for a in args {
                collect_free_locals(a, params, out);
            }
        }
        Expr::Switch { subject, arms, .. } => {
            collect_free_locals(subject, params, out);
            for arm in arms {
                collect_free_locals(&arm.body, params, out);
            }
        }
        Expr::Catch {
            expr,
            fallback,
            ..
        } => {
            collect_free_locals(expr, params, out);
            collect_free_locals(fallback, params, out);
        }
        Expr::StringInterp(parts) => {
            for p in parts {
                if let hir::InterpPart::Expr(ex) = p {
                    collect_free_locals(ex, params, out);
                }
            }
        }
        _ => {}
    }
}

fn subst_locals(e: &mut Expr, subst: &IndexMap<String, Expr>) {
    match e {
        Expr::Local(n) | Expr::Param(n) => {
            if let Some(rep) = subst.get(n) {
                *e = rep.clone();
            }
        }
        Expr::Unary { expr, .. }
        | Expr::OptionalSome(expr)
        | Expr::TryOptional(expr)
        | Expr::Cast { expr, .. } => subst_locals(expr, subst),
        Expr::Binary { left, right, .. } => {
            subst_locals(left, subst);
            subst_locals(right, subst);
        }
        Expr::Call { callee, args, .. } => {
            if let Callee::Method { receiver, .. } = callee {
                subst_locals(receiver, subst);
            }
            for a in args {
                subst_locals(a, subst);
            }
        }
        Expr::Member { base, .. } => subst_locals(base, subst),
        Expr::Index { base, index, .. } => {
            subst_locals(base, subst);
            subst_locals(index, subst);
        }
        Expr::IfExpr {
            cond,
            then_e,
            else_e,
            ..
        } => {
            subst_locals(cond, subst);
            subst_locals(then_e, subst);
            subst_locals(else_e, subst);
        }
        Expr::ArrayLit { elems, .. } => {
            for el in elems {
                subst_locals(el, subst);
            }
        }
        Expr::DictLit { entries } => {
            for (k, v) in entries {
                subst_locals(k, subst);
                subst_locals(v, subst);
            }
        }
        Expr::Construct { args, .. } => {
            for (_, a) in args {
                subst_locals(a, subst);
            }
        }
        Expr::EnumCase { args, .. } => {
            for a in args {
                subst_locals(a, subst);
            }
        }
        Expr::Switch { subject, arms, .. } => {
            subst_locals(subject, subst);
            for arm in arms {
                subst_locals(&mut arm.body, subst);
            }
        }
        Expr::Catch {
            expr,
            fallback,
            ..
        } => {
            subst_locals(expr, subst);
            subst_locals(fallback, subst);
        }
        Expr::StringInterp(parts) => {
            for p in parts {
                if let hir::InterpPart::Expr(ex) = p {
                    subst_locals(ex, subst);
                }
            }
        }
        _ => {}
    }
}

fn aexpr_span(e: &AExpr) -> Span {
    match e {
        AExpr::LiteralInt { span, .. }
        | AExpr::LiteralFloat { span, .. }
        | AExpr::LiteralString { span, .. }
        | AExpr::LiteralBool { span, .. }
        | AExpr::LiteralNil { span }
        | AExpr::LiteralChar { span, .. }
        | AExpr::Ident { span, .. }
        | AExpr::CompileConst { span, .. }
        | AExpr::SelfExpr { span, .. }
        | AExpr::Array { span, .. }
        | AExpr::Dictionary { span, .. }
        | AExpr::Unary { span, .. }
        | AExpr::Binary { span, .. }
        | AExpr::Call { span, .. }
        | AExpr::Member { span, .. }
        | AExpr::Index { span, .. }
        | AExpr::Assign { span, .. }
        | AExpr::If { span, .. }
        | AExpr::Switch { span, .. }
        | AExpr::Closure { span, .. }
        | AExpr::Lambda { span, .. }
        | AExpr::Try { span, .. }
        | AExpr::Await { span, .. }
        | AExpr::Catch { span, .. }
        | AExpr::DoCatch { span, .. }
        | AExpr::Ternary { span, .. }
        | AExpr::NilCoalesce { span, .. }
        | AExpr::ForceUnwrap { span, .. }
        | AExpr::Paren { span, .. }
        | AExpr::Cast { span, .. }
        | AExpr::Is { span, .. }
        | AExpr::Range { span, .. }
        | AExpr::Unsafe { span, .. }
        | AExpr::Error { span } => *span,
    }
}

fn aexpr_kind(e: &AExpr) -> &'static str {
    match e {
        AExpr::Array { .. } => "array literal",
        AExpr::Dictionary { .. } => "dictionary literal",
        AExpr::Index { .. } => "subscript",
        AExpr::Closure { .. } | AExpr::Lambda { .. } => "closure",
        AExpr::Range { .. } => "range",
        AExpr::Cast { .. } => "cast",
        AExpr::Is { .. } => "is-check",
        AExpr::NilCoalesce { .. } => "nil-coalesce",
        AExpr::ForceUnwrap { .. } => "force-unwrap",
        AExpr::DoCatch { .. } => "do-catch expression",
        AExpr::Unsafe { .. } => "unsafe",
        AExpr::Assign { .. } => "assignment",
        _ => "expression",
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
            | "Dictionary"
            | "Set"
            | "Environment"
            | "Env"
            | "Identity"
            | "Args"
            | "File"
            | "Instant"
            | "Process"
            | "ProcessHandle"
            | "Result"
            | "JSON"
            | "TOON"
            | "Http"
            | "HttpServer"
            | "Log"
            | "CLI"
            | "Crypto"
            | "HMAC"
            | "Base64"
            | "Hex"
            | "SecureRandom"
            | "Hash"
            | "Arena"
            | "Mutex"
            | "TcpStream"
            | "Task"
            | "Channel"
            | "WaitGroup"
            | "TaskGroup"
            | "CancellationToken"
            | "Regex"
            | "SQLManager"
            | "SQLConnection"
            | "MainActor"
            | "UInt8"
            | "UInt16"
            | "UInt32"
            | "UInt64"
            | "Float32"
            | "Float64"
            | "Decimal"
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
        Ty::String
            | Ty::Int
            | Ty::Int64
            | Ty::UInt
            | Ty::Double
            | Ty::Float
            | Ty::Bool
            | Ty::Char
    ) || matches!(ty, Ty::Named(n) if n == "Decimal")
        || ty.is_error()
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
