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
        /// Field names held weakly (no strong retain; cleared on deinit).
        weak_fields: Vec<String>,
        unowned_fields: Vec<String>,
        is_actor: bool,
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
    /// Phase 10: async function (await allowed; body still sync-shaped ABI).
    pub is_async: bool,
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
        /// When set, `name` holds a Task handle; `await name` yields this result type.
        async_let_result: Option<Ty>,
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
    For {
        /// Loop variable name.
        name: String,
        /// Start bound.
        start: Expr,
        /// End bound (`..<` exclusive, `...` inclusive).
        end: Expr,
        /// When true, iterate `start...end` (inclusive).
        inclusive: bool,
        body: Body,
    },
    /// `for x in arr` — iterate Int array elements.
    ForEach {
        name: String,
        array: Expr,
        body: Body,
    },
    Break,
    Continue,
    Defer {
        body: Body,
    },
    Die {
        message: String,
    },
    /// Explicit `{ ... }` block — scopes `#defer` to the block (Phase 1.1).
    Scope {
        body: Body,
    },
    /// `@run func()` — spawn zero-arg void function on the executor.
    Run {
        func: String,
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
    /// Reference to superclass for static dispatch (`super.method`).
    Super,
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
    ArrayLit {
        elems: Vec<Expr>,
        elem_ty: Ty,
    },
    DictLit {
        entries: Vec<(Expr, Expr)>,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        ty: Ty,
    },
    /// Reference to a top-level / synthetic function (closures, method refs).
    /// `captures` are prepended as leading arguments when the function is called
    /// (Phase 1.2 closure env — values retained/copied at the creation site).
    FuncRef {
        name: String,
        ty: Ty,
        captures: Vec<Expr>,
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
    /// Static call to parent class method (`super.foo`).
    SuperMethod {
        parent: String,
        name: String,
    },
    Builtin(String),
    /// Spawn zero-arg function `name` on the executor (`ll_task_spawn` / `_ret`).
    TaskSpawn {
        name: String,
        /// When true, use `ll_task_spawn_ret` (pointer-returning worker).
        returns_value: bool,
    },
    /// `TaskGroup.add(fn)` — spawn zero-arg void `name` into the group (arg[0] = group).
    TaskGroupAdd {
        name: String,
    },
    /// `HttpServer.serve(port, handler)` — handler is `(String) -> String` mangled name.
    HttpServe {
        handler: String,
    },
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
