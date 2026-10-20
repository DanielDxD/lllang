//! Typed high-level IR (Phase 2 subset).

use crate::types::Ty;

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub functions: Vec<Function>,
    pub types: Vec<TypeDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDef {
    Struct {
        name: String,
        fields: Vec<(String, Ty)>,
        methods: Vec<Function>,
        conforms: Vec<String>,
    },
    Class {
        name: String,
        fields: Vec<(String, Ty)>,
        methods: Vec<Function>,
        parent: Option<String>,
        conforms: Vec<String>,
        virtual_methods: Vec<String>,
    },
    Enum {
        name: String,
        cases: Vec<EnumCase>,
        conforms: Vec<String>,
    },
    Protocol {
        name: String,
        methods: Vec<ProtoMethod>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCase {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtoMethod {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub default_body: Option<Body>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub throws: Option<Ty>,
    pub is_method: bool,
    pub self_ty: Option<Ty>,
    pub is_virtual: bool,
    pub is_override: bool,
    pub body: Option<Body>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        ty: Ty,
        init: Expr,
    },
    Var {
        name: String,
        ty: Ty,
        init: Option<Expr>,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>),
    Throw(Expr),
    Guard {
        cond: Expr,
        else_body: Body,
    },
    If {
        cond: Expr,
        then_body: Body,
        else_body: Option<Body>,
    },
    While {
        cond: Expr,
        body: Body,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Nil,
    StringLit(String),
    StringInterp(Vec<InterpPart>),
    Local(String),
    Param(String),
    Self_,
    Call {
        callee: Callee,
        args: Vec<Expr>,
        ty: Ty,
        is_try: TryKind,
    },
    Member {
        base: Box<Expr>,
        name: String,
        ty: Ty,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        ty: Ty,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        ty: Ty,
    },
    IfExpr {
        cond: Box<Expr>,
        then_e: Box<Expr>,
        else_e: Box<Expr>,
        ty: Ty,
    },
    Switch {
        subject: Box<Expr>,
        arms: Vec<SwitchArm>,
        ty: Ty,
    },
    OptionalSome(Box<Expr>),
    OptionalNone(Ty),
    TryOptional(Box<Expr>),
    Catch {
        expr: Box<Expr>,
        binder: String,
        fallback: Box<Expr>,
        ty: Ty,
    },
    Construct {
        ty_name: String,
        args: Vec<(String, Expr)>,
        ty: Ty,
    },
    EnumCase {
        enum_name: String,
        case: String,
        args: Vec<Expr>,
        ty: Ty,
    },
    Cast {
        expr: Box<Expr>,
        to: Ty,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterpPart {
    Lit(String),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Callee {
    Func(String),
    Method {
        receiver: Box<Expr>,
        name: String,
    },
    Virtual {
        receiver: Box<Expr>,
        name: String,
    },
    Builtin(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryKind {
    None,
    Try,
    TryOptional,
    TryForce,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    LitInt(i64),
    LitBool(bool),
    EnumCase {
        case: String,
        bindings: Vec<String>,
    },
    Ident(String),
    OptionalSome(String),
    OptionalNone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}
