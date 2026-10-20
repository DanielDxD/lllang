//! Lower HIR to MIR.
//!
//! ## Throwing call ABI (Phase 2)
//!
//! Functions with `throws(E)` take an extra parameter `__err_out: **void` (pointer to error object slot).
//! On success the callee stores `null` into `*__err_out` and returns the normal value (or void).
//! On throw the callee stores the error payload pointer and returns zero/poison for value returns.
//! Callers pass the address of a local `void*` slot; after the call, non-null means an error occurred.

use crate::{
    BinOp, Block, Callee, Function, Inst, InterpPart, Local, MirTy, Program, SwitchKey, Terminator,
    TypeDef, UnOp, Value,
};
use indexmap::IndexMap;
use lllang_sema::hir::{
    self, BinaryOp, Body, Callee as HirCallee, Expr, Function as HirFunction, InterpPart as HirInterp,
    Module, Pattern, Stmt, TryKind, TypeDef as HirTypeDef, UnaryOp,
};
use lllang_sema::types::Ty;

pub fn lower(module: &Module) -> Program {
    let mut l = Lowerer::new(module);
    l.lower_module()
}

pub fn emit_mir(program: &Program) -> String {
    let mut out = String::new();
    for td in &program.types {
        match td {
            TypeDef::Struct { name, fields } => {
                out.push_str(&format!("struct {name} {{"));
                for (n, t) in fields {
                    out.push_str(&format!(" {n}: {t:?},"));
                }
                out.push_str(" }\n");
            }
            TypeDef::Class { name, fields, parent } => {
                out.push_str(&format!("class {name} extends {parent:?} {{"));
                for (n, t) in fields {
                    out.push_str(&format!(" {n}: {t:?},"));
                }
                out.push_str(" }\n");
            }
            TypeDef::Enum { name, cases } => {
                out.push_str(&format!("enum {name} {{"));
                for c in cases {
                    out.push_str(&format!(" {}({:?}),", c.name, c.fields));
                }
                out.push_str(" }\n");
            }
        }
    }
    for f in &program.functions {
        out.push_str(&format!(
            "fn {}({:?}) -> {:?} throws {:?}\n",
            f.name, f.params, f.ret, f.throws
        ));
        for b in &f.blocks {
            out.push_str(&format!("  bb{}:\n", b.id));
            for s in &b.stmts {
                out.push_str(&format!("    {s:?}\n"));
            }
            out.push_str(&format!("    {term:?}\n", term = b.term));
        }
        out.push('\n');
    }
    out
}

struct Lowerer<'a> {
    module: &'a Module,
    type_fields: IndexMap<String, Vec<(String, MirTy)>>,
    enum_cases: IndexMap<String, Vec<(String, Vec<(String, MirTy)>)>>,
    is_class: IndexMap<String, bool>,
}

struct FuncLower<'a> {
    lower: &'a Lowerer<'a>,
    blocks: Vec<Block>,
    cur_block: usize,
    next_local: usize,
    next_block: usize,
    scopes: Vec<IndexMap<String, (Local, MirTy)>>,
    /// Locals that hold refcounted values and need Release at epilogue.
    refcounted: Vec<Local>,
    /// Per-function error out slot local (address taken for throwing calls).
    err_slot: Local,
    err_slot_ty: MirTy,
    func_throws: Option<MirTy>,
    /// Receiver type for the function being lowered (methods only).
    self_ty: Option<MirTy>,
}

impl<'a> Lowerer<'a> {
    fn new(module: &'a Module) -> Self {
        let mut type_fields = IndexMap::new();
        let mut enum_cases = IndexMap::new();
        let mut is_class = IndexMap::new();
        for td in &module.types {
            match td {
                HirTypeDef::Struct { name, fields, .. } => {
                    type_fields.insert(
                        name.clone(),
                        fields.iter().map(|(n, t)| (n.clone(), MirTy::from_hir(t))).collect(),
                    );
                    is_class.insert(name.clone(), false);
                }
                HirTypeDef::Class { name, fields, .. } => {
                    type_fields.insert(
                        name.clone(),
                        fields.iter().map(|(n, t)| (n.clone(), MirTy::from_hir(t))).collect(),
                    );
                    is_class.insert(name.clone(), true);
                }
                HirTypeDef::Enum { name, cases, .. } => {
                    enum_cases.insert(
                        name.clone(),
                        cases
                            .iter()
                            .map(|c| {
                                (
                                    c.name.clone(),
                                    c.fields
                                        .iter()
                                        .map(|(n, t)| (n.clone(), MirTy::from_hir(t)))
                                        .collect(),
                                )
                            })
                            .collect(),
                    );
                }
                HirTypeDef::Protocol { .. } => {}
            }
        }
        Self {
            module,
            type_fields,
            enum_cases,
            is_class,
        }
    }

    fn lower_module(&mut self) -> Program {
        let types: Vec<TypeDef> = self.module.types.iter().map(map_type_def).collect();
        let functions: Vec<Function> = self
            .module
            .functions
            .iter()
            .filter(|f| f.body.is_some())
            .map(|f| self.lower_function(f))
            .collect();
        Program { functions, types }
    }

    fn lower_function(&self, f: &HirFunction) -> Function {
        let name = mir_func_name(&f.name);
        let mut params: Vec<(String, MirTy)> = Vec::new();
        // Methods take an implicit `self` receiver as the first parameter.
        if f.is_method {
            if let Some(self_ty) = &f.self_ty {
                params.push(("self".into(), MirTy::from_hir(self_ty)));
            }
        }
        params.extend(
            f.params
                .iter()
                .map(|(n, t)| (n.clone(), MirTy::from_hir(t))),
        );
        if f.throws.is_some() {
            params.push(("__err_out".into(), MirTy::Named("__err_out".into())));
        }

        let ret = MirTy::from_hir(&f.ret);
        let throws = f.throws.as_ref().map(MirTy::from_hir);

        let mut fl = FuncLower {
            lower: self,
            blocks: vec![Block {
                id: 0,
                stmts: vec![],
                term: Terminator::Unreachable,
            }],
            cur_block: 0,
            next_local: 0,
            next_block: 1,
            scopes: vec![IndexMap::new()],
            refcounted: vec![],
            err_slot: Local(0),
            err_slot_ty: MirTy::Named("void*".into()),
            func_throws: throws.clone(),
            self_ty: f.self_ty.as_ref().map(MirTy::from_hir),
        };

        let mut param_locals = Vec::new();
        for (pname, pty) in &params {
            if pname == "__err_out" {
                continue;
            }
            let loc = fl.alloc_local(pty.clone());
            param_locals.push((pname.clone(), loc));
            fl.scopes[0].insert(pname.clone(), (loc, pty.clone()));
            if pty.is_refcounted() {
                fl.refcounted.push(loc);
            }
        }

        // Project primary-constructor / stored fields into named locals for method bodies.
        if f.is_method {
            if let Some((_, self_loc)) = param_locals.iter().find(|(n, _)| n == "self") {
                let self_loc = *self_loc;
                let sty = params
                    .iter()
                    .find(|(n, _)| n == "self")
                    .map(|(_, t)| t.clone())
                    .unwrap_or(MirTy::Void);
                let type_name = match &sty {
                    MirTy::Struct(n) | MirTy::Class(n) => Some(n.as_str()),
                    _ => None,
                };
                if let Some(tn) = type_name {
                    if let Some(fields) = self.type_fields.get(tn) {
                        for (i, (fname, fty)) in fields.iter().enumerate() {
                            let floc = fl.alloc_local(fty.clone());
                            fl.emit(Inst::GetField {
                                dst: floc,
                                base: Value::Local(self_loc),
                                index: i,
                                ty: fty.clone(),
                            });
                            fl.scopes[0].insert(fname.clone(), (floc, fty.clone()));
                            if fty.is_refcounted() {
                                fl.refcounted.push(floc);
                            }
                        }
                    }
                }
            }
        }

        if let Some(body) = &f.body {
            fl.lower_body(body, &ret);
        }

        Function {
            name,
            params,
            param_locals,
            ret,
            throws,
            blocks: fl.blocks,
        }
    }
}

fn map_type_def(td: &HirTypeDef) -> TypeDef {
    match td {
        HirTypeDef::Struct { name, fields, .. } => TypeDef::Struct {
            name: name.clone(),
            fields: fields.iter().map(|(n, t)| (n.clone(), MirTy::from_hir(t))).collect(),
        },
        HirTypeDef::Class { name, fields, parent, .. } => TypeDef::Class {
            name: name.clone(),
            fields: fields.iter().map(|(n, t)| (n.clone(), MirTy::from_hir(t))).collect(),
            parent: parent.clone(),
        },
        HirTypeDef::Enum { name, cases, .. } => TypeDef::Enum {
            name: name.clone(),
            cases: cases
                .iter()
                .map(|c| crate::EnumCase {
                    name: c.name.clone(),
                    fields: c.fields.iter().map(|(n, t)| (n.clone(), MirTy::from_hir(t))).collect(),
                })
                .collect(),
        },
        HirTypeDef::Protocol { name, .. } => TypeDef::Struct {
            name: name.clone(),
            fields: vec![],
        },
    }
}

fn mir_func_name(hir_name: &str) -> String {
    let simple = hir_name.rsplit('.').next().unwrap_or(hir_name);
    if simple == "main" {
        "ll_main".into()
    } else {
        hir_name.replace('.', "_")
    }
}

fn map_binop(op: BinaryOp) -> BinOp {
    match op {
        BinaryOp::Add => BinOp::Add,
        BinaryOp::Sub => BinOp::Sub,
        BinaryOp::Mul => BinOp::Mul,
        BinaryOp::Div => BinOp::Div,
        BinaryOp::Mod => BinOp::Mod,
        BinaryOp::Eq => BinOp::Eq,
        BinaryOp::NotEq => BinOp::NotEq,
        BinaryOp::Lt => BinOp::Lt,
        BinaryOp::Le => BinOp::Le,
        BinaryOp::Gt => BinOp::Gt,
        BinaryOp::Ge => BinOp::Ge,
        BinaryOp::And => BinOp::And,
        BinaryOp::Or => BinOp::Or,
    }
}

fn map_unop(op: UnaryOp) -> UnOp {
    match op {
        UnaryOp::Not => UnOp::Not,
        UnaryOp::Neg => UnOp::Neg,
        UnaryOp::Pos => UnOp::Pos,
    }
}

impl FuncLower<'_> {
    fn fresh_block(&mut self) -> usize {
        let id = self.next_block;
        self.next_block += 1;
        self.blocks.push(Block {
            id,
            stmts: vec![],
            term: Terminator::Unreachable,
        });
        id
    }

    fn emit(&mut self, inst: Inst) {
        self.blocks[self.cur_block].stmts.push(inst);
    }

    fn set_term(&mut self, term: Terminator) {
        self.blocks[self.cur_block].term = term;
    }

    fn switch_block(&mut self, id: usize) {
        self.cur_block = id;
    }

    fn alloc_local(&mut self, ty: MirTy) -> Local {
        let loc = Local(self.next_local);
        self.next_local += 1;
        self.emit(Inst::Alloca { dst: loc, ty: ty.clone() });
        loc
    }

    fn bind_local(&mut self, name: &str, loc: Local, ty: MirTy) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), (loc, ty));
    }

    fn lookup(&self, name: &str) -> Option<(Local, MirTy)> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    fn track_refcounted(&mut self, loc: Local, ty: &MirTy) {
        if ty.is_refcounted() && !self.refcounted.contains(&loc) {
            self.refcounted.push(loc);
        }
    }

    fn lower_body(&mut self, body: &Body, ret_ty: &MirTy) {
        let stmts = &body.stmts;
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i + 1 == stmts.len();
            // Implicit return: trailing expression in a non-void function.
            if is_last && *ret_ty != MirTy::Void {
                if let Stmt::Expr(e) = stmt {
                    let v = self.lower_expr(e, ret_ty);
                    self.finish_return(Some(v), ret_ty);
                    return;
                }
            }
            self.lower_stmt(stmt, ret_ty);
        }
        // Initial terminator is Unreachable — treat it as "not yet set".
        if !matches!(
            self.blocks[self.cur_block].term,
            Terminator::Return(_) | Terminator::Throw(_)
        ) {
            self.finish_return(None, ret_ty);
        }
    }

    fn finish_return(&mut self, val: Option<Value>, ret_ty: &MirTy) {
        let returned = match &val {
            Some(Value::Local(l)) => Some(*l),
            _ => None,
        };
        for loc in self.refcounted.clone() {
            // Don't release the value being returned — ownership transfers to the caller.
            if returned == Some(loc) {
                continue;
            }
            self.emit(Inst::Release(Value::Local(loc)));
        }
        if ret_ty == &MirTy::Void {
            self.set_term(Terminator::Return(None));
        } else {
            self.set_term(Terminator::Return(val));
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt, ret_ty: &MirTy) {
        match stmt {
            Stmt::Let { name, ty, init } => {
                let mty = MirTy::from_hir(ty);
                let loc = self.alloc_local(mty.clone());
                let v = self.lower_expr(init, &mty);
                self.emit(Inst::Store { dst: loc, val: v });
                self.track_refcounted(loc, &mty);
                self.bind_local(name, loc, mty);
            }
            Stmt::Var { name, ty, init } => {
                let mty = MirTy::from_hir(ty);
                let loc = self.alloc_local(mty.clone());
                if let Some(e) = init {
                    let v = self.lower_expr(e, &mty);
                    self.emit(Inst::Store { dst: loc, val: v });
                }
                self.track_refcounted(loc, &mty);
                self.bind_local(name, loc, mty);
            }
            Stmt::Assign { target, value } => {
                let (base, field_idx, fty) = self.resolve_lvalue(target);
                let v = self.lower_expr(value, &fty);
                if let Some((loc, _)) = base {
                    self.emit(Inst::SetField {
                        base: Value::Local(loc),
                        index: field_idx,
                        val: v,
                    });
                }
            }
            Stmt::Expr(e) => {
                let ty = expr_mir_ty(e);
                let _ = self.lower_expr(e, &ty);
            }
            Stmt::Return(e) => {
                let v = e.as_ref().map(|ex| self.lower_expr(ex, ret_ty));
                self.finish_return(v, ret_ty);
            }
            Stmt::Throw(e) => {
                let et = expr_mir_ty(e);
                let v = self.lower_expr(e, &et);
                self.set_term(Terminator::Throw(v));
            }
            Stmt::Guard { cond, else_body } => {
                let cty = MirTy::Bool;
                let cv = self.lower_expr(cond, &cty);
                let then_b = self.fresh_block();
                let else_b = self.fresh_block();
                let merge = self.fresh_block();
                self.set_term(Terminator::Branch {
                    cond: cv,
                    then_b,
                    else_b,
                });
                self.switch_block(then_b);
                self.set_term(Terminator::Jump(merge));
                self.switch_block(else_b);
                self.lower_body(else_body, ret_ty);
                if !matches!(
                    self.blocks[self.cur_block].term,
                    Terminator::Return(_) | Terminator::Throw(_)
                ) {
                    self.set_term(Terminator::Jump(merge));
                }
                self.switch_block(merge);
            }
            Stmt::If { cond, then_body, else_body } => {
                let cv = self.lower_expr(cond, &MirTy::Bool);
                let then_b = self.fresh_block();
                let else_b = self.fresh_block();
                let merge = self.fresh_block();
                self.set_term(Terminator::Branch {
                    cond: cv,
                    then_b,
                    else_b,
                });
                self.switch_block(then_b);
                self.lower_body(then_body, ret_ty);
                if !matches!(
                    self.blocks[self.cur_block].term,
                    Terminator::Return(_) | Terminator::Throw(_)
                ) {
                    self.set_term(Terminator::Jump(merge));
                }
                self.switch_block(else_b);
                if let Some(eb) = else_body {
                    self.lower_body(eb, ret_ty);
                }
                if !matches!(
                    self.blocks[self.cur_block].term,
                    Terminator::Return(_) | Terminator::Throw(_)
                ) {
                    self.set_term(Terminator::Jump(merge));
                }
                self.switch_block(merge);
            }
            Stmt::While { cond, body } => {
                let head = self.fresh_block();
                let body_b = self.fresh_block();
                let exit = self.fresh_block();
                self.set_term(Terminator::Jump(head));
                self.switch_block(head);
                let cv = self.lower_expr(cond, &MirTy::Bool);
                self.set_term(Terminator::Branch {
                    cond: cv,
                    then_b: body_b,
                    else_b: exit,
                });
                self.switch_block(body_b);
                self.lower_body(body, ret_ty);
                if !matches!(
                    self.blocks[self.cur_block].term,
                    Terminator::Return(_) | Terminator::Throw(_)
                ) {
                    self.set_term(Terminator::Jump(head));
                }
                self.switch_block(exit);
            }
        }
    }

    fn resolve_lvalue(&mut self, expr: &Expr) -> (Option<(Local, MirTy)>, usize, MirTy) {
        match expr {
            Expr::Local(name) | Expr::Param(name) => {
                let (loc, ty) = self.lookup(name).expect("assign local");
                (Some((loc, ty.clone())), 0, ty)
            }
            Expr::Member { base, name, ty } => {
                let base_ty = expr_mir_ty(base);
                let (bloc, _) = self.lower_expr_to_local(base, &base_ty);
                let idx = self.lower.get_type_field_index(&base_ty, name);
                (Some((bloc, base_ty)), idx, MirTy::from_hir(ty))
            }
            _ => panic!("unsupported lvalue {:?}", expr),
        }
    }

    fn lower_expr(&mut self, expr: &Expr, expected: &MirTy) -> Value {
        match expr {
            Expr::Int(n) => Value::ConstInt(*n),
            Expr::Float(f) => Value::ConstFloat(*f),
            Expr::Bool(b) => Value::ConstBool(*b),
            Expr::Char(c) => Value::ConstChar(*c as u32),
            Expr::Nil => Value::Nil,
            Expr::StringLit(s) => {
                let loc = self.alloc_local(MirTy::String);
                self.emit(Inst::StringLit {
                    dst: loc,
                    value: s.clone(),
                });
                self.track_refcounted(loc, &MirTy::String);
                Value::Local(loc)
            }
            Expr::StringInterp(parts) => {
                let loc = self.alloc_local(MirTy::String);
                let mir_parts: Vec<InterpPart> = parts
                    .iter()
                    .map(|p| match p {
                        HirInterp::Lit(s) => InterpPart::Lit(s.clone()),
                        HirInterp::Expr(e) => {
                            let t = expr_mir_ty(e);
                            InterpPart::Value(self.lower_expr(e, &t))
                        }
                    })
                    .collect();
                self.emit(Inst::StringInterp {
                    dst: loc,
                    parts: mir_parts,
                });
                self.track_refcounted(loc, &MirTy::String);
                Value::Local(loc)
            }
            Expr::Local(name) | Expr::Param(name) => {
                let (src, ty) = self
                    .lookup(name)
                    .unwrap_or_else(|| panic!("local `{name}` not in scope"));
                if ty.is_refcounted() {
                    self.emit(Inst::Retain(Value::Local(src)));
                }
                Value::Local(src)
            }
            Expr::Self_ => {
                let (src, ty) = self.lookup("self").expect("self");
                if ty.is_refcounted() {
                    self.emit(Inst::Retain(Value::Local(src)));
                }
                Value::Local(src)
            }
            Expr::Unary { op, expr, ty } => {
                let inner = self.lower_expr(expr, expected);
                let dst = self.alloc_local(MirTy::from_hir(ty));
                self.emit(Inst::Unary {
                    dst,
                    op: map_unop(*op),
                    val: inner,
                    ty: MirTy::from_hir(ty),
                });
                Value::Local(dst)
            }
            Expr::Binary { op, left, right, ty } => {
                let l = self.lower_expr(left, expected);
                let r = self.lower_expr(right, expected);
                let dst = self.alloc_local(MirTy::from_hir(ty));
                self.emit(Inst::Binary {
                    dst,
                    op: map_binop(*op),
                    left: l,
                    right: r,
                    ty: MirTy::from_hir(ty),
                });
                Value::Local(dst)
            }
            Expr::IfExpr { cond, then_e, else_e, ty } => {
                let cv = self.lower_expr(cond, &MirTy::Bool);
                let mty = MirTy::from_hir(ty);
                let then_b = self.fresh_block();
                let else_b = self.fresh_block();
                let merge = self.fresh_block();
                self.set_term(Terminator::Branch {
                    cond: cv,
                    then_b,
                    else_b,
                });
                self.switch_block(then_b);
                let tv = self.lower_expr(then_e, &mty);
                let tloc = self.materialize_to_local(tv, &mty);
                self.set_term(Terminator::Jump(merge));
                self.switch_block(else_b);
                let ev = self.lower_expr(else_e, &mty);
                let eloc = self.materialize_to_local(ev, &mty);
                self.set_term(Terminator::Jump(merge));
                self.switch_block(merge);
                let dst = self.alloc_local(mty.clone());
                // Phi simplified: use merge block with two predecessors storing — for MVP use tloc only
                // Phase 2: emit select via duplicate stores in predecessor blocks
                self.emit(Inst::Store {
                    dst,
                    val: Value::Local(tloc),
                });
                let _ = eloc;
                Value::Local(dst)
            }
            Expr::OptionalSome(inner) => {
                let inner_ty = optional_inner(expected).unwrap_or(MirTy::Int);
                let v = self.lower_expr(inner, &inner_ty);
                let dst = self.alloc_local(expected.clone());
                self.emit(Inst::OptionalSome {
                    dst,
                    inner: v,
                    inner_ty,
                });
                Value::Local(dst)
            }
            Expr::OptionalNone(ty) => {
                let inner_ty = MirTy::from_hir(ty);
                let dst = self.alloc_local(MirTy::Optional(Box::new(inner_ty.clone())));
                self.emit(Inst::OptionalNone { dst, inner_ty });
                Value::Local(dst)
            }
            Expr::TryOptional(inner) => {
                // `try? f(...)` — forward TryOptional into the throwing call ABI.
                if let Expr::Call {
                    callee,
                    args,
                    ty,
                    ..
                } = inner.as_ref()
                {
                    return self.lower_call(callee, args, ty, TryKind::TryOptional);
                }
                // Non-call: treat as optional passthrough (already optional).
                let inner_ty = expr_mir_ty(inner);
                self.lower_expr(inner, &inner_ty)
            }
            Expr::Catch { expr, binder, fallback, ty } => {
                self.lower_catch(expr, binder, fallback, ty)
            }
            Expr::Call {
                callee,
                args,
                ty,
                is_try,
            } => self.lower_call(callee, args, ty, *is_try),
            Expr::Member { base, name, ty } => {
                let base_ty = match base.as_ref() {
                    Expr::Local(n) | Expr::Param(n) => self
                        .lookup(n)
                        .map(|(_, t)| t)
                        .unwrap_or_else(|| expr_mir_ty(base)),
                    Expr::Self_ => self.self_ty.clone().unwrap_or_else(|| expr_mir_ty(base)),
                    _ => expr_mir_ty(base),
                };
                let bv = self.lower_expr(base, &base_ty);
                if is_property_access(&base_ty, name) {
                    return self.lower_builtin_property(bv, name, ty);
                }
                let dst = self.alloc_local(MirTy::from_hir(ty));
                let idx = self.lower.get_type_field_index(&base_ty, name);
                self.emit(Inst::GetField {
                    dst,
                    base: bv,
                    index: idx,
                    ty: MirTy::from_hir(ty),
                });
                Value::Local(dst)
            }
            Expr::Construct { ty_name, args, ty } => self.lower_construct(ty_name, args, ty),
            Expr::EnumCase { enum_name, case, args, ty } => {
                self.lower_enum_case(enum_name, case, args, ty)
            }
            Expr::Switch { subject, arms, ty } => self.lower_switch(subject, arms, ty),
            Expr::Cast { expr, to } => {
                let from_ty = expr_mir_ty(expr);
                self.lower_expr(expr, &from_ty)
            }
        }
    }

    fn materialize_to_local(&mut self, v: Value, ty: &MirTy) -> Local {
        match v {
            Value::Local(l) => l,
            other => {
                let loc = self.alloc_local(ty.clone());
                self.emit(Inst::Store { dst: loc, val: other });
                loc
            }
        }
    }

    fn lower_expr_to_local(&mut self, expr: &Expr, ty: &MirTy) -> (Local, MirTy) {
        let v = self.lower_expr(expr, ty);
        let loc = self.materialize_to_local(v, ty);
        (loc, ty.clone())
    }

    fn lower_call(
        &mut self,
        callee: &HirCallee,
        args: &[Expr],
        ret_ty: &Ty,
        try_kind: TryKind,
    ) -> Value {
        if let HirCallee::Builtin(id) = callee {
            if id == "unwrap_optional" && args.len() == 1 {
                let opt_ty = expr_mir_ty(&args[0]);
                let opt = self.lower_expr(&args[0], &opt_ty);
                let inner_ty = MirTy::from_hir(ret_ty);
                let dst = self.alloc_local(inner_ty.clone());
                self.emit(Inst::UnwrapOptional {
                    dst,
                    opt,
                    inner_ty: inner_ty.clone(),
                    force: true,
                });
                if inner_ty.is_refcounted() {
                    self.track_refcounted(dst, &inner_ty);
                }
                return Value::Local(dst);
            }
            if id == "optional_is_some" && args.len() == 1 {
                let opt_ty = expr_mir_ty(&args[0]);
                let opt = self.lower_expr(&args[0], &opt_ty);
                let dst = self.alloc_local(MirTy::Bool);
                // Represent as UnwrapOptional with force=false writing presence into Bool dst via
                // OptionalSome tag check — use Call to runtime helper for portability.
                self.emit(Inst::Call {
                    dst: Some(dst),
                    callee: Callee::Builtin("ll_optional_is_some".into()),
                    args: vec![opt],
                    ty: MirTy::Bool,
                });
                return Value::Local(dst);
            }
        }

        let mret = MirTy::from_hir(ret_ty);
        let mut mir_args: Vec<Value> = Vec::new();
        // Instance methods: pass receiver as first argument.
        match callee {
            HirCallee::Method { receiver, .. } | HirCallee::Virtual { receiver, .. } => {
                let rty = expr_mir_ty(receiver);
                mir_args.push(self.lower_expr(receiver, &rty));
            }
            _ => {}
        }
        for a in args {
            let t = expr_mir_ty(a);
            mir_args.push(self.lower_expr(a, &t));
        }

        let (callee_mir, throws) = self.resolve_callee(callee);
        let is_throwing = throws.is_some();

        if is_throwing {
            return self.lower_throwing_call(callee_mir, mir_args, &mret, try_kind);
        }

        let dst = if mret == MirTy::Void {
            None
        } else {
            Some(self.alloc_local(mret.clone()))
        };
        self.emit(Inst::Call {
            dst,
            callee: callee_mir,
            args: mir_args,
            ty: mret.clone(),
        });
        if let Some(d) = dst {
            if mret.is_refcounted() {
                self.track_refcounted(d, &mret);
            }
            Value::Local(d)
        } else {
            Value::Nil
        }
    }

    fn lower_throwing_call(
        &mut self,
        callee: Callee,
        args: Vec<Value>,
        mret: &MirTy,
        try_kind: TryKind,
    ) -> Value {
        let err_slot = self.alloc_local(MirTy::Named("void*".into()));
        let err_ptr = self.alloc_local(MirTy::Named("__err_out".into()));
        // err_ptr = &err_slot — represented as special local; LLVM lowers address of err_slot

        let mut call_args = args;
        call_args.push(Value::Local(err_ptr));

        let dst = if *mret == MirTy::Void {
            None
        } else {
            Some(self.alloc_local(mret.clone()))
        };

        self.emit(Inst::Call {
            dst,
            callee,
            args: call_args,
            ty: mret.clone(),
        });

        let flag = self.alloc_local(MirTy::Bool);
        self.emit(Inst::LoadErrFlag { dst: flag });

        match try_kind {
            TryKind::TryOptional => {
                let opt_ty = MirTy::Optional(Box::new(mret.clone()));
                let opt = self.alloc_local(opt_ty);
                let some_b = self.fresh_block();
                let none_b = self.fresh_block();
                let merge = self.fresh_block();
                self.set_term(Terminator::Branch {
                    cond: Value::Local(flag),
                    then_b: none_b,
                    else_b: some_b,
                });
                self.switch_block(some_b);
                if let Some(d) = dst {
                    self.emit(Inst::OptionalSome {
                        dst: opt,
                        inner: Value::Local(d),
                        inner_ty: mret.clone(),
                    });
                }
                self.set_term(Terminator::Jump(merge));
                self.switch_block(none_b);
                self.emit(Inst::OptionalNone {
                    dst: opt,
                    inner_ty: mret.clone(),
                });
                self.set_term(Terminator::Jump(merge));
                self.switch_block(merge);
                Value::Local(opt)
            }
            TryKind::Try | TryKind::TryForce => {
                if let Some(d) = dst {
                    Value::Local(d)
                } else {
                    Value::Nil
                }
            }
            TryKind::None => {
                if let Some(d) = dst {
                    Value::Local(d)
                } else {
                    Value::Nil
                }
            }
        }
    }

    fn lower_catch(
        &mut self,
        expr: &Expr,
        binder: &str,
        fallback: &Expr,
        ty: &Ty,
    ) -> Value {
        let mty = MirTy::from_hir(ty);
        // Evaluate the throwing expression (records err flag via LoadErrFlag after call).
        let ok_v = if let Expr::Call {
            callee,
            args,
            ty: call_ty,
            ..
        } = expr
        {
            self.lower_call(callee, args, call_ty, TryKind::Try)
        } else {
            self.lower_expr(expr, &mty)
        };

        let flag = self.alloc_local(MirTy::Bool);
        self.emit(Inst::LoadErrFlag { dst: flag });

        let ok_b = self.fresh_block();
        let err_b = self.fresh_block();
        let merge = self.fresh_block();
        let result = self.alloc_local(mty.clone());

        // flag true ⇒ error occurred
        self.set_term(Terminator::Branch {
            cond: Value::Local(flag),
            then_b: err_b,
            else_b: ok_b,
        });

        self.switch_block(ok_b);
        let ok_loc = self.materialize_to_local(ok_v, &mty);
        self.emit(Inst::Store {
            dst: result,
            val: Value::Local(ok_loc),
        });
        self.set_term(Terminator::Jump(merge));

        self.switch_block(err_b);
        let _ = binder; // binder available for future typed catch payloads
        let fb = self.lower_expr(fallback, &mty);
        let fb_loc = self.materialize_to_local(fb, &mty);
        self.emit(Inst::Store {
            dst: result,
            val: Value::Local(fb_loc),
        });
        self.set_term(Terminator::Jump(merge));

        self.switch_block(merge);
        Value::Local(result)
    }

    fn resolve_callee(&self, callee: &HirCallee) -> (Callee, Option<MirTy>) {
        match callee {
            HirCallee::Func(name) => {
                let throws = self
                    .lower
                    .module
                    .functions
                    .iter()
                    .find(|f| f.name == *name || mir_func_name(&f.name) == mir_func_name(name))
                    .and_then(|f| f.throws.as_ref().map(MirTy::from_hir));
                (Callee::Func(mir_func_name(name)), throws)
            }
            HirCallee::Builtin(id) => {
                let rt = map_builtin(id);
                (Callee::Builtin(rt), None)
            }
            HirCallee::Method { receiver, name } => {
                let rt = static_method_name(receiver, name, self);
                (Callee::Func(rt), None)
            }
            HirCallee::Virtual { receiver, name } => {
                let rt = static_method_name(receiver, name, self);
                (Callee::Func(rt), None)
            }
        }
    }

    fn lower_builtin_property(&mut self, base: Value, name: &str, ty: &Ty) -> Value {
        let mty = MirTy::from_hir(ty);
        if name == "isEmpty" {
            let dst = self.alloc_local(mty.clone());
            self.emit(Inst::Call {
                dst: Some(dst),
                callee: Callee::Builtin("ll_string_is_empty".into()),
                args: vec![base],
                ty: mty,
            });
            return Value::Local(dst);
        }
        let dst = self.alloc_local(mty);
        Value::Local(dst)
    }

    fn lower_construct(&mut self, ty_name: &str, args: &[(String, Expr)], ty: &Ty) -> Value {
        let fields = self.lower.type_fields.get(ty_name).cloned().unwrap_or_default();
        let mut field_vals = vec![Value::Nil; fields.len()];
        for (label, expr) in args {
            let idx = fields.iter().position(|(n, _)| n == label).unwrap_or(0);
            let fty = fields.get(idx).map(|(_, t)| t.clone()).unwrap_or(MirTy::Int);
            field_vals[idx] = self.lower_expr(expr, &fty);
        }
        let is_class = self.lower.is_class.get(ty_name).copied().unwrap_or(false);
        let dst = self.alloc_local(MirTy::from_hir(ty));
        self.emit(Inst::Construct {
            dst,
            ty_name: ty_name.to_string(),
            fields: field_vals,
            is_class,
        });
        if is_class {
            self.track_refcounted(dst, &MirTy::Class(ty_name.into()));
        }
        Value::Local(dst)
    }

    fn lower_enum_case(
        &mut self,
        enum_name: &str,
        case: &str,
        args: &[Expr],
        ty: &Ty,
    ) -> Value {
        let cases = self.lower.enum_cases.get(enum_name).cloned().unwrap_or_default();
        let case_idx = cases.iter().position(|(n, _)| n == case).unwrap_or(0);
        let case_fields = &cases[case_idx].1;
        let mut payload = vec![];
        for (i, arg) in args.iter().enumerate() {
            let fty = case_fields.get(i).map(|(_, t)| t.clone()).unwrap_or(MirTy::Int);
            payload.push(self.lower_expr(arg, &fty));
        }
        let dst = self.alloc_local(MirTy::from_hir(ty));
        // Encode case index in the name so codegen can set the tag: Enum#case#idx
        self.emit(Inst::Construct {
            dst,
            ty_name: format!("{enum_name}#{case}#{case_idx}"),
            fields: payload,
            is_class: false,
        });
        Value::Local(dst)
    }

    fn lower_switch(&mut self, subject: &Expr, arms: &[hir::SwitchArm], ty: &Ty) -> Value {
        let sub_ty = match subject {
            Expr::Local(n) | Expr::Param(n) => self
                .lookup(n)
                .map(|(_, t)| t)
                .unwrap_or_else(|| expr_mir_ty(subject)),
            _ => expr_mir_ty(subject),
        };
        let sv = self.lower_expr(subject, &sub_ty);
        let mty = MirTy::from_hir(ty);
        let merge = self.fresh_block();
        let result_loc = self.alloc_local(mty.clone());

        // Optional: branch on presence bit instead of switching on the aggregate.
        if matches!(sub_ty, MirTy::Optional(_)) {
            let flag = self.alloc_local(MirTy::Bool);
            self.emit(Inst::Call {
                dst: Some(flag),
                callee: Callee::Builtin("ll_optional_is_some".into()),
                args: vec![sv.clone()],
                ty: MirTy::Bool,
            });
            let some_b = self.fresh_block();
            let none_b = self.fresh_block();
            self.set_term(Terminator::Branch {
                cond: Value::Local(flag),
                then_b: some_b,
                else_b: none_b,
            });

            let mut saw_some = false;
            let mut saw_none = false;
            for arm in arms {
                let is_some = match &arm.pattern {
                    Pattern::OptionalSome(_) => true,
                    Pattern::EnumCase { case, .. } if case == "some" => true,
                    _ => false,
                };
                let is_none = match &arm.pattern {
                    Pattern::OptionalNone => true,
                    Pattern::EnumCase { case, .. } if case == "none" => true,
                    _ => false,
                };
                let target = if is_some {
                    saw_some = true;
                    some_b
                } else if is_none {
                    saw_none = true;
                    none_b
                } else {
                    continue;
                };
                self.switch_block(target);
                bind_pattern(self, &arm.pattern, &sub_ty, sv.clone());
                let rv = self.lower_expr(&arm.body, &mty);
                let rl = self.materialize_to_local(rv, &mty);
                self.emit(Inst::Store {
                    dst: result_loc,
                    val: Value::Local(rl),
                });
                self.set_term(Terminator::Jump(merge));
            }
            if !saw_some {
                self.switch_block(some_b);
                self.set_term(Terminator::Jump(merge));
            }
            if !saw_none {
                self.switch_block(none_b);
                self.set_term(Terminator::Jump(merge));
            }
            self.switch_block(merge);
            return Value::Local(result_loc);
        }

        let default_b = self.fresh_block();
        let mut arm_blocks = vec![];
        let mut switch_arms = vec![];

        for arm in arms {
            let bb = self.fresh_block();
            arm_blocks.push((bb, arm));
            let key = pattern_key(&arm.pattern, &sub_ty);
            if let Some(k) = key {
                switch_arms.push((k, bb));
            }
        }

        // For enums, switch on the tag field.
        let discr = if matches!(sub_ty, MirTy::Enum(_)) {
            let tag = self.alloc_local(MirTy::Int32);
            self.emit(Inst::EnumTag {
                dst: tag,
                base: sv.clone(),
            });
            Value::Local(tag)
        } else {
            sv.clone()
        };

        self.set_term(Terminator::Switch {
            subject: discr,
            arms: switch_arms,
            default: default_b,
        });

        for (bb, arm) in arm_blocks {
            self.switch_block(bb);
            if let Some(g) = &arm.guard {
                let gv = self.lower_expr(g, &MirTy::Bool);
                let pass = self.fresh_block();
                let skip = self.fresh_block();
                self.set_term(Terminator::Branch {
                    cond: gv,
                    then_b: pass,
                    else_b: skip,
                });
                self.switch_block(pass);
            }
            bind_pattern(self, &arm.pattern, &sub_ty, sv.clone());
            let rv = self.lower_expr(&arm.body, &mty);
            let rl = self.materialize_to_local(rv, &mty);
            self.emit(Inst::Store {
                dst: result_loc,
                val: Value::Local(rl),
            });
            self.set_term(Terminator::Jump(merge));
        }

        self.switch_block(default_b);
        self.set_term(Terminator::Jump(merge));
        self.switch_block(merge);
        Value::Local(result_loc)
    }
}

impl Lowerer<'_> {
    fn get_type_field_index(&self, ty: &MirTy, field: &str) -> usize {
        let name = match ty {
            MirTy::Struct(n) | MirTy::Class(n) => n,
            _ => return 0,
        };
        self.type_fields
            .get(name)
            .and_then(|fs| fs.iter().position(|(n, _)| n == field))
            .unwrap_or(0)
    }
}

fn map_builtin(id: &str) -> String {
    match id {
        "print" => "ll_print".into(),
        "int_from_string" => "ll_int_from_string".into(),
        "string_is_empty" => "ll_string_is_empty".into(),
        "double_sqrt" => "ll_double_sqrt".into(),
        other => other.to_string(),
    }
}

fn static_method_name(receiver: &Expr, method: &str, fl: &FuncLower<'_>) -> String {
    let type_name = receiver_static_type(receiver, fl).unwrap_or_else(|| "Unknown".into());
    format!("{type_name}_{method}")
}

fn receiver_static_type(expr: &Expr, fl: &FuncLower<'_>) -> Option<String> {
    if matches!(expr, Expr::Self_) {
        if let Some(ty) = &fl.self_ty {
            return match ty {
                MirTy::Struct(n) | MirTy::Class(n) | MirTy::Named(n) => Some(n.clone()),
                _ => None,
            };
        }
    }
    let ty = match expr {
        Expr::Local(n) | Expr::Param(n) => fl.lookup(n).map(|(_, t)| t),
        _ => Some(expr_mir_ty(expr)),
    };
    match ty {
        Some(MirTy::Struct(n) | MirTy::Class(n) | MirTy::Named(n)) => Some(n),
        _ => {
            if let Expr::Construct { ty_name, .. } = expr {
                Some(ty_name.clone())
            } else {
                None
            }
        }
    }
}

fn is_property_access(base_ty: &MirTy, name: &str) -> bool {
    name == "isEmpty" && matches!(base_ty, MirTy::String)
}

fn optional_inner(ty: &MirTy) -> Option<MirTy> {
    match ty {
        MirTy::Optional(inner) => Some((**inner).clone()),
        _ => None,
    }
}

fn expr_mir_ty(expr: &Expr) -> MirTy {
    match expr {
        Expr::Int(_) => MirTy::Int,
        Expr::Float(_) => MirTy::Double,
        Expr::Bool(_) => MirTy::Bool,
        Expr::Char(_) => MirTy::Char,
        Expr::Nil => MirTy::Optional(Box::new(MirTy::Void)),
        Expr::StringLit(_) | Expr::StringInterp(_) => MirTy::String,
        Expr::Local(_) | Expr::Param(_) | Expr::Self_ => MirTy::Int,
        Expr::Unary { ty, .. } | Expr::Binary { ty, .. } | Expr::IfExpr { ty, .. } | Expr::Member { ty, .. }
        | Expr::Call { ty, .. } | Expr::Construct { ty, .. } | Expr::EnumCase { ty, .. }
        | Expr::Switch { ty, .. } | Expr::Catch { ty, .. } => MirTy::from_hir(ty),
        Expr::OptionalNone(t) => MirTy::Optional(Box::new(MirTy::from_hir(t))),
        Expr::OptionalSome(e) => MirTy::Optional(Box::new(expr_mir_ty(e))),
        Expr::TryOptional(e) => MirTy::Optional(Box::new(expr_mir_ty(e))),
        Expr::Cast { to, .. } => MirTy::from_hir(to),
    }
}

fn pattern_key(pattern: &Pattern, sub_ty: &MirTy) -> Option<SwitchKey> {
    match pattern {
        Pattern::LitInt(n) => Some(SwitchKey::Int(*n)),
        Pattern::LitBool(b) => Some(SwitchKey::Bool(*b)),
        Pattern::EnumCase { case, .. } => {
            if let MirTy::Enum(name) = sub_ty {
                // tag index resolved at codegen; use hash placeholder 0
                let _ = (name, case);
                Some(SwitchKey::Tag(0))
            } else if matches!(sub_ty, MirTy::Optional(_)) {
                match case.as_str() {
                    "some" => Some(SwitchKey::Tag(1)),
                    "none" => Some(SwitchKey::Tag(0)),
                    _ => Some(SwitchKey::Tag(0)),
                }
            } else {
                None
            }
        }
        Pattern::OptionalSome(_) => Some(SwitchKey::Tag(1)),
        Pattern::OptionalNone => Some(SwitchKey::Tag(0)),
        _ => None,
    }
}

fn bind_pattern(fl: &mut FuncLower<'_>, pattern: &Pattern, sub_ty: &MirTy, subject: Value) {
    match pattern {
        Pattern::Ident(name) | Pattern::OptionalSome(name) => {
            let inner = optional_inner(sub_ty).unwrap_or_else(|| sub_ty.clone());
            let loc = fl.alloc_local(inner.clone());
            if matches!(pattern, Pattern::OptionalSome(_)) {
                fl.emit(Inst::UnwrapOptional {
                    dst: loc,
                    opt: subject,
                    inner_ty: inner.clone(),
                    force: true,
                });
            } else {
                fl.emit(Inst::Store { dst: loc, val: subject });
            }
            fl.bind_local(name, loc, inner);
        }
        Pattern::EnumCase { case, bindings, .. } => {
            if matches!(sub_ty, MirTy::Optional(_)) && case == "some" {
                if let Some(bname) = bindings.first() {
                    let inner = optional_inner(sub_ty).unwrap_or(MirTy::Int);
                    let loc = fl.alloc_local(inner.clone());
                    fl.emit(Inst::UnwrapOptional {
                        dst: loc,
                        opt: subject,
                        inner_ty: inner.clone(),
                        force: true,
                    });
                    fl.bind_local(bname, loc, inner);
                }
                return;
            }
            for (i, bname) in bindings.iter().enumerate() {
                let loc = fl.alloc_local(MirTy::Int);
                fl.emit(Inst::EnumPayload {
                    dst: loc,
                    base: subject.clone(),
                    case_index: 0,
                    field_index: i,
                    ty: MirTy::Int,
                });
                fl.bind_local(bname, loc, MirTy::Int);
            }
        }
        _ => {}
    }
}
