//! Compile-time `$` / `#declare` / `#if` expansion (Phase 9 MVP).

use crate::ast::*;
use crate::diagnostic::{codes, Diagnostic, Severity};
use crate::span::Span;
use std::collections::HashMap;

/// Value of a compile-time constant.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Bool(bool),
    Int(i64),
    String(String),
}

impl ConstValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConstValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn to_expr(&self, span: Span) -> Expr {
        match self {
            ConstValue::Bool(v) => Expr::LiteralBool { span, value: *v },
            ConstValue::Int(v) => Expr::LiteralInt {
                span,
                value: v.to_string(),
            },
            ConstValue::String(v) => Expr::LiteralString {
                span,
                parts: vec![StringPart::Text(v.clone())],
            },
        }
    }
}

/// Names that `#declare` must not override.
const SYSTEM_NAMES: &[&str] = &[
    "OS",
    "ARCH",
    "DEBUG",
    "RELEASE",
    "SDK_VERSION",
    "VERSION",
    "BUILD_TIME",
    "BUILD_NUMBER",
    "BUILD_HASH",
    "BUILD_TAG",
    "BUILD_TYPE",
    "BUILD_ENV",
    "MAIN_ACTOR",
];

#[derive(Debug, Clone)]
pub struct CompileEnv {
    values: HashMap<String, ConstValue>,
}

impl CompileEnv {
    pub fn system(debug: bool) -> Self {
        let mut values = HashMap::new();
        let os = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else {
            std::env::consts::ARCH
        };
        values.insert("OS".into(), ConstValue::String(os.into()));
        values.insert("ARCH".into(), ConstValue::String(arch.into()));
        values.insert("DEBUG".into(), ConstValue::Bool(debug));
        values.insert("RELEASE".into(), ConstValue::Bool(!debug));
        values.insert(
            "SDK_VERSION".into(),
            ConstValue::String(env!("CARGO_PKG_VERSION").into()),
        );
        values.insert(
            "VERSION".into(),
            ConstValue::String(
                std::env::var("LANG_APP_VERSION").unwrap_or_else(|_| "0.1.0".into()),
            ),
        );
        values.insert(
            "BUILD_TYPE".into(),
            ConstValue::String(if debug { "debug" } else { "release" }.into()),
        );
        values.insert(
            "BUILD_ENV".into(),
            ConstValue::String(std::env::var("LANG_BUILD_ENV").unwrap_or_else(|_| {
                if debug {
                    "development".into()
                } else {
                    "production".into()
                }
            })),
        );
        values.insert(
            "BUILD_TIME".into(),
            ConstValue::String(
                std::env::var("LANG_BUILD_TIME").unwrap_or_else(|_| "unknown".into()),
            ),
        );
        values.insert(
            "BUILD_NUMBER".into(),
            ConstValue::String(std::env::var("LANG_BUILD_NUMBER").unwrap_or_else(|_| "0".into())),
        );
        values.insert(
            "BUILD_HASH".into(),
            ConstValue::String(
                std::env::var("LANG_BUILD_HASH").unwrap_or_else(|_| "unknown".into()),
            ),
        );
        values.insert(
            "BUILD_TAG".into(),
            ConstValue::String(std::env::var("LANG_BUILD_TAG").unwrap_or_else(|_| {
                if debug {
                    "debug".into()
                } else {
                    "release".into()
                }
            })),
        );
        values.insert("MAIN_ACTOR".into(), ConstValue::Bool(true));
        Self { values }
    }

    pub fn get(&self, name: &str) -> Option<&ConstValue> {
        self.values.get(name)
    }

    pub fn declare(&mut self, name: &str, value: ConstValue) -> Result<(), String> {
        if SYSTEM_NAMES.contains(&name) {
            return Err(format!("cannot `#declare` system constant `${name}`"));
        }
        self.values.insert(name.into(), value);
        Ok(())
    }
}

/// Expand `#declare` / `#if` / `$IDENT` in a parsed file.
pub fn expand_file(file: File, env: &mut CompileEnv) -> (File, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let mut decls = Vec::new();
    for d in file.decls {
        match d {
            Decl::CompileDeclare { span, name, value } => match expr_to_const(&value) {
                Some(v) => {
                    if let Err(msg) = env.declare(&name, v) {
                        diags.push(diag_err(span, msg));
                    }
                }
                None => diags.push(diag_err(
                    span,
                    format!("`#declare ${name}` requires a bool/int/string literal"),
                )),
            },
            other => decls.push(expand_decl(other, env, &mut diags)),
        }
    }
    (
        File {
            span: file.span,
            decls,
        },
        diags,
    )
}

fn diag_err(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        code: codes::UNEXPECTED_TOKEN.into(),
        severity: Severity::Error,
        message: message.into(),
        span,
        file: None,
    }
}

fn expr_to_const(e: &Expr) -> Option<ConstValue> {
    match e {
        Expr::LiteralBool { value, .. } => Some(ConstValue::Bool(*value)),
        Expr::LiteralInt { value, .. } => Some(ConstValue::Int(value.parse().ok()?)),
        Expr::LiteralString { parts, .. } => {
            let mut s = String::new();
            for p in parts {
                match p {
                    StringPart::Text(t) => s.push_str(t),
                    StringPart::Interpolation(_) => return None,
                }
            }
            Some(ConstValue::String(s))
        }
        _ => None,
    }
}

fn expand_decl(d: Decl, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> Decl {
    match d {
        Decl::Func(mut f) => {
            f.body = expand_func_body(f.body, env, diags);
            Decl::Func(f)
        }
        Decl::Struct(mut s) => {
            s.members = s
                .members
                .into_iter()
                .map(|m| expand_member(m, env, diags))
                .collect();
            Decl::Struct(s)
        }
        Decl::Class(mut c) => {
            c.members = c
                .members
                .into_iter()
                .map(|m| expand_member(m, env, diags))
                .collect();
            Decl::Class(c)
        }
        Decl::Enum(mut e) => {
            e.members = e
                .members
                .into_iter()
                .map(|m| expand_member(m, env, diags))
                .collect();
            Decl::Enum(e)
        }
        Decl::Extension(mut e) => {
            e.members = e
                .members
                .into_iter()
                .map(|m| expand_member(m, env, diags))
                .collect();
            Decl::Extension(e)
        }
        Decl::Let(mut l) => {
            for b in &mut l.bindings {
                if let Some(init) = b.initializer.take() {
                    b.initializer = Some(expand_expr(init, env, diags));
                }
            }
            Decl::Let(l)
        }
        Decl::Var(mut v) => {
            for b in &mut v.bindings {
                if let Some(init) = b.initializer.take() {
                    b.initializer = Some(expand_expr(init, env, diags));
                }
            }
            Decl::Var(v)
        }
        Decl::Const(mut c) => {
            c.value = expand_expr(c.value, env, diags);
            Decl::Const(c)
        }
        other => other,
    }
}

fn expand_func_body(body: FuncBody, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> FuncBody {
    match body {
        FuncBody::Block(b) => FuncBody::Block(expand_block(b, env, diags)),
        FuncBody::Expr(e) => FuncBody::Expr(Box::new(expand_expr(*e, env, diags))),
        FuncBody::Missing => FuncBody::Missing,
    }
}

fn expand_member(m: MemberDecl, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> MemberDecl {
    match m {
        MemberDecl::Func(mut f) => {
            f.body = expand_func_body(f.body, env, diags);
            MemberDecl::Func(f)
        }
        MemberDecl::Init(mut i) => {
            i.body = expand_func_body(i.body, env, diags);
            MemberDecl::Init(i)
        }
        MemberDecl::Deinit(mut d) => {
            d.body = expand_block(d.body, env, diags);
            MemberDecl::Deinit(d)
        }
        MemberDecl::Let(mut l) => {
            for b in &mut l.bindings {
                if let Some(init) = b.initializer.take() {
                    b.initializer = Some(expand_expr(init, env, diags));
                }
            }
            MemberDecl::Let(l)
        }
        MemberDecl::Var(mut v) => {
            for b in &mut v.bindings {
                if let Some(init) = b.initializer.take() {
                    b.initializer = Some(expand_expr(init, env, diags));
                }
            }
            MemberDecl::Var(v)
        }
        MemberDecl::Nested(d) => MemberDecl::Nested(Box::new(expand_decl(*d, env, diags))),
    }
}

fn expand_block(block: Block, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> Block {
    let mut stmts = Vec::new();
    for s in block.stmts {
        stmts.extend(expand_stmt(s, env, diags));
    }
    Block {
        span: block.span,
        stmts,
    }
}

fn expand_stmt(stmt: Stmt, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> Vec<Stmt> {
    match stmt {
        Stmt::CompileIf {
            span,
            condition,
            then_body,
            else_body,
        } => {
            let cond = expand_expr(condition, env, diags);
            let take_then = match eval_bool(&cond) {
                Some(b) => b,
                None => {
                    diags.push(diag_err(
                        span,
                        "`#if` condition must be a compile-time bool (`$DEBUG`, …)",
                    ));
                    true
                }
            };
            let body = if take_then {
                then_body
            } else {
                else_body.unwrap_or(Block {
                    span,
                    stmts: vec![],
                })
            };
            vec![Stmt::Block(expand_block(body, env, diags))]
        }
        Stmt::Expr { span, expr } => vec![Stmt::Expr {
            span,
            expr: expand_expr(expr, env, diags),
        }],
        Stmt::Let(mut l) => {
            for b in &mut l.bindings {
                if let Some(init) = b.initializer.take() {
                    b.initializer = Some(expand_expr(init, env, diags));
                }
            }
            vec![Stmt::Let(l)]
        }
        Stmt::Var(mut v) => {
            for b in &mut v.bindings {
                if let Some(init) = b.initializer.take() {
                    b.initializer = Some(expand_expr(init, env, diags));
                }
            }
            vec![Stmt::Var(v)]
        }
        Stmt::Return { span, value } => vec![Stmt::Return {
            span,
            value: value.map(|e| expand_expr(e, env, diags)),
        }],
        Stmt::Throw { span, value } => vec![Stmt::Throw {
            span,
            value: expand_expr(value, env, diags),
        }],
        Stmt::Guard {
            span,
            condition,
            else_body,
        } => {
            let condition = match condition {
                GuardCondition::Expr(e) => GuardCondition::Expr(expand_expr(e, env, diags)),
                GuardCondition::Let(p, e) => GuardCondition::Let(p, expand_expr(e, env, diags)),
            };
            vec![Stmt::Guard {
                span,
                condition,
                else_body: expand_block(else_body, env, diags),
            }]
        }
        Stmt::If {
            span,
            arms,
            else_branch,
        } => vec![Stmt::If {
            span,
            arms: arms
                .into_iter()
                .map(|a| IfArm {
                    span: a.span,
                    condition: expand_if_condition(a.condition, env, diags),
                    body: expand_if_body(a.body, env, diags),
                })
                .collect(),
            else_branch: else_branch.map(|e| expand_if_else(e, env, diags)),
        }],
        Stmt::While {
            span,
            label,
            condition,
            body,
        } => vec![Stmt::While {
            span,
            label,
            condition: expand_expr(condition, env, diags),
            body: expand_block(body, env, diags),
        }],
        Stmt::For {
            span,
            label,
            pattern,
            sequence,
            filter,
            body,
        } => vec![Stmt::For {
            span,
            label,
            pattern,
            sequence: expand_expr(sequence, env, diags),
            filter: filter.map(|e| expand_expr(e, env, diags)),
            body: expand_block(body, env, diags),
        }],
        Stmt::DoCatch {
            span,
            body,
            catches,
        } => vec![Stmt::DoCatch {
            span,
            body: expand_block(body, env, diags),
            catches: catches
                .into_iter()
                .map(|c| CatchClause {
                    span: c.span,
                    pattern: c.pattern,
                    body: expand_catch_body(c.body, env, diags),
                })
                .collect(),
        }],
        Stmt::Defer { span, body } => {
            let body = match body {
                DeferBody::Expr(e) => DeferBody::Expr(expand_expr(e, env, diags)),
                DeferBody::Block(b) => DeferBody::Block(expand_block(b, env, diags)),
            };
            vec![Stmt::Defer { span, body }]
        }
        Stmt::Block(b) => vec![Stmt::Block(expand_block(b, env, diags))],
        Stmt::Run { span, expr } => vec![Stmt::Run {
            span,
            expr: expand_expr(expr, env, diags),
        }],
        other => vec![other],
    }
}

fn expand_if_condition(
    c: IfCondition,
    env: &CompileEnv,
    diags: &mut Vec<Diagnostic>,
) -> IfCondition {
    match c {
        IfCondition::Expr(e) => IfCondition::Expr(expand_expr(e, env, diags)),
        IfCondition::Let(p, e) => IfCondition::Let(p, e.map(|x| expand_expr(x, env, diags))),
        IfCondition::Var(p, e) => IfCondition::Var(p, e.map(|x| expand_expr(x, env, diags))),
    }
}

fn expand_if_body(b: IfBody, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> IfBody {
    match b {
        IfBody::Block(block) => IfBody::Block(expand_block(block, env, diags)),
        IfBody::Arrow(e) => IfBody::Arrow(Box::new(expand_expr(*e, env, diags))),
    }
}

fn expand_if_else(e: IfElse, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> IfElse {
    match e {
        IfElse::Block(b) => IfElse::Block(expand_block(b, env, diags)),
        IfElse::Expr(x) => IfElse::Expr(Box::new(expand_expr(*x, env, diags))),
        IfElse::If(x) => IfElse::If(Box::new(expand_expr(*x, env, diags))),
    }
}

fn expand_catch_body(b: CatchBody, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> CatchBody {
    match b {
        CatchBody::Block(block) => CatchBody::Block(expand_block(block, env, diags)),
        CatchBody::Arrow(e) => CatchBody::Arrow(Box::new(expand_expr(*e, env, diags))),
    }
}

fn eval_bool(e: &Expr) -> Option<bool> {
    match e {
        Expr::LiteralBool { value, .. } => Some(*value),
        _ => None,
    }
}

fn expand_expr(expr: Expr, env: &CompileEnv, diags: &mut Vec<Diagnostic>) -> Expr {
    match expr {
        Expr::CompileConst { span, name } => match env.get(&name) {
            Some(v) => v.to_expr(span),
            None => {
                diags.push(diag_err(
                    span,
                    format!("unknown compile-time constant `${name}`"),
                ));
                Expr::Error { span }
            }
        },
        Expr::Unary { span, op, operand } => Expr::Unary {
            span,
            op,
            operand: Box::new(expand_expr(*operand, env, diags)),
        },
        Expr::Binary {
            span,
            op,
            left,
            right,
        } => Expr::Binary {
            span,
            op,
            left: Box::new(expand_expr(*left, env, diags)),
            right: Box::new(expand_expr(*right, env, diags)),
        },
        Expr::Call {
            span,
            callee,
            args,
            try_kind,
        } => Expr::Call {
            span,
            callee: Box::new(expand_expr(*callee, env, diags)),
            args: args
                .into_iter()
                .map(|a| CallArg {
                    span: a.span,
                    label: a.label,
                    value: expand_expr(a.value, env, diags),
                })
                .collect(),
            try_kind,
        },
        Expr::Member {
            span,
            base,
            name,
            optional,
        } => Expr::Member {
            span,
            base: Box::new(expand_expr(*base, env, diags)),
            name,
            optional,
        },
        Expr::Index {
            span,
            base,
            indices,
            optional,
        } => Expr::Index {
            span,
            base: Box::new(expand_expr(*base, env, diags)),
            indices: indices
                .into_iter()
                .map(|e| expand_expr(e, env, diags))
                .collect(),
            optional,
        },
        Expr::Assign {
            span,
            target,
            op,
            value,
        } => Expr::Assign {
            span,
            target: Box::new(expand_expr(*target, env, diags)),
            op,
            value: Box::new(expand_expr(*value, env, diags)),
        },
        Expr::Paren { span, expr } => Expr::Paren {
            span,
            expr: Box::new(expand_expr(*expr, env, diags)),
        },
        Expr::Array { span, elements } => Expr::Array {
            span,
            elements: elements
                .into_iter()
                .map(|e| expand_expr(e, env, diags))
                .collect(),
        },
        Expr::Dictionary { span, entries } => Expr::Dictionary {
            span,
            entries: entries
                .into_iter()
                .map(|(k, v)| (expand_expr(k, env, diags), expand_expr(v, env, diags)))
                .collect(),
        },
        Expr::Is {
            span,
            expr,
            ty,
            binding,
        } => Expr::Is {
            span,
            expr: Box::new(expand_expr(*expr, env, diags)),
            ty,
            binding,
        },
        Expr::Cast {
            span,
            expr,
            ty,
            kind,
        } => Expr::Cast {
            span,
            expr: Box::new(expand_expr(*expr, env, diags)),
            ty,
            kind,
        },
        other => other,
    }
}
