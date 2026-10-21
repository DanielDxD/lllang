//! Mid-level IR for LLLang (Phase 2).

mod lower;

pub use lower::{emit_mir, lower};

use lllang_sema::types::Ty;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub functions: Vec<Function>,
    pub types: Vec<TypeDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, MirTy)>,
    /// Parameter locals in order (excludes `__err_out`), matching LLVM argument order.
    pub param_locals: Vec<(String, Local)>,
    pub ret: MirTy,
    pub throws: Option<MirTy>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: usize,
    pub stmts: Vec<Inst>,
    pub term: Terminator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Local(pub usize);

#[derive(Debug, Clone, PartialEq)]
pub enum MirTy {
    Void,
    Never,
    Bool,
    Int,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt,
    Float,
    Double,
    Char,
    String,
    Optional(Box<MirTy>),
    Array(Box<MirTy>),
    Tuple(Vec<MirTy>),
    Named(String),
    Class(String),
    Struct(String),
    Enum(String),
    Error,
}

impl MirTy {
    pub fn from_hir(ty: &Ty) -> Self {
        match ty {
            Ty::Void => MirTy::Void,
            Ty::Never => MirTy::Never,
            Ty::Bool => MirTy::Bool,
            Ty::Int => MirTy::Int,
            Ty::Int8 => MirTy::Int8,
            Ty::Int16 => MirTy::Int16,
            Ty::Int32 => MirTy::Int32,
            Ty::Int64 => MirTy::Int64,
            Ty::UInt => MirTy::UInt,
            Ty::Float => MirTy::Float,
            Ty::Double => MirTy::Double,
            Ty::Char => MirTy::Char,
            Ty::String => MirTy::String,
            Ty::Optional(inner) => MirTy::Optional(Box::new(MirTy::from_hir(inner))),
            Ty::Array(inner) => MirTy::Array(Box::new(MirTy::from_hir(inner))),
            Ty::Tuple(elems) => MirTy::Tuple(elems.iter().map(MirTy::from_hir).collect()),
            Ty::Named(n) => MirTy::Named(n.clone()),
            Ty::Func { .. } => MirTy::Named("<func>".into()),
            Ty::Protocol(n) => MirTy::Class(n.clone()),
            Ty::Class(n) => MirTy::Class(n.clone()),
            Ty::Struct(n) => MirTy::Struct(n.clone()),
            Ty::Enum(n) => MirTy::Enum(n.clone()),
            Ty::Error => MirTy::Error,
        }
    }

    pub fn is_refcounted(&self) -> bool {
        matches!(
            self,
            MirTy::String | MirTy::Array(_) | MirTy::Class(_)
        ) || matches!(self, MirTy::Optional(inner) if inner.is_refcounted())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Local(Local),
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstChar(u32),
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Callee {
    Func(String),
    Builtin(String),
    /// Dynamic dispatch: `args[0]` is `self`; call `vtable[slot]`.
    Virtual {
        class: String,
        method: String,
        slot: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
    Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterpPart {
    Lit(String),
    Value(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inst {
    Alloca { dst: Local, ty: MirTy },
    Store { dst: Local, val: Value },
    Load { dst: Local, src: Local },
    Call {
        dst: Option<Local>,
        callee: Callee,
        args: Vec<Value>,
        ty: MirTy,
    },
    Binary {
        dst: Local,
        op: BinOp,
        left: Value,
        right: Value,
        ty: MirTy,
    },
    Unary {
        dst: Local,
        op: UnOp,
        val: Value,
        ty: MirTy,
    },
    Retain(Value),
    Release(Value),
    Construct {
        dst: Local,
        ty_name: String,
        fields: Vec<Value>,
        is_class: bool,
    },
    GetField {
        dst: Local,
        base: Value,
        index: usize,
        ty: MirTy,
    },
    SetField {
        base: Value,
        index: usize,
        val: Value,
    },
    EnumTag { dst: Local, base: Value },
    EnumPayload {
        dst: Local,
        base: Value,
        case_index: usize,
        field_index: usize,
        ty: MirTy,
    },
    StringLit { dst: Local, value: String },
    StringInterp { dst: Local, parts: Vec<InterpPart> },
    OptionalSome {
        dst: Local,
        inner: Value,
        inner_ty: MirTy,
    },
    OptionalNone { dst: Local, inner_ty: MirTy },
    UnwrapOptional {
        dst: Local,
        opt: Value,
        inner_ty: MirTy,
        force: bool,
    },
    /// Store into the throwing error out-parameter slot (see lower module docs).
    SetErrOut { val: Value },
    /// Load whether err_out is non-null after a throwing call.
    LoadErrFlag { dst: Local },
    /// Load the error payload pointer from `__err_out` into `dst` (pointer / boxed).
    LoadErrValue { dst: Local },
}

#[derive(Debug, Clone, PartialEq)]
pub enum SwitchKey {
    Int(i64),
    Bool(bool),
    Tag(i32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Return(Option<Value>),
    Throw(Value),
    Branch {
        cond: Value,
        then_b: usize,
        else_b: usize,
    },
    Jump(usize),
    Switch {
        subject: Value,
        arms: Vec<(SwitchKey, usize)>,
        default: usize,
    },
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDef {
    Struct {
        name: String,
        fields: Vec<(String, MirTy)>,
    },
    Class {
        name: String,
        fields: Vec<(String, MirTy)>,
        parent: Option<String>,
        virtual_methods: Vec<String>,
    },
    Enum {
        name: String,
        cases: Vec<EnumCase>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCase {
    pub name: String,
    pub fields: Vec<(String, MirTy)>,
}
