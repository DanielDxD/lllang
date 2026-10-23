//! Abstract syntax tree for LLLang.

use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct File {
    pub span: Span,
    pub decls: Vec<Decl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Namespace(NamespaceDecl),
    Import(ImportDecl),
    Struct(StructDecl),
    Class(ClassDecl),
    Enum(EnumDecl),
    Protocol(ProtocolDecl),
    Extension(ExtensionDecl),
    Func(FuncDecl),
    Let(LetDecl),
    Var(VarDecl),
    Const(ConstDecl),
    Attribute(Attribute),
    /// `#declare $NAME lit` — compile-time constant binding (expanded before sema).
    CompileDeclare {
        span: Span,
        name: String,
        value: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub span: Span,
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Open,
    Public,
    Internal,
    Private,
    Protected,
    Fileprivate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub path: QualifiedName,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub path: QualifiedName,
    pub alias: Option<String>,
    pub wildcard: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedName {
    pub span: Span,
    pub segments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParam {
    pub span: Span,
    pub name: String,
    pub constraint: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericConstraint {
    pub span: Span,
    pub subject: Type,
    pub bound: GenericBound,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenericBound {
    Type(Type),
    SameType(Type),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrimaryParam {
    pub span: Span,
    pub access: Option<Access>,
    pub modifiers: ParamModifiers,
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParamModifiers {
    pub storage: Option<StorageKind>,
    pub inout: bool,
    pub weak: bool,
    pub unowned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    Let,
    Var,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InheritanceClause {
    pub span: Span,
    pub items: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub primary_ctor: Vec<PrimaryParam>,
    pub inheritance: Option<InheritanceClause>,
    pub where_clause: Vec<GenericConstraint>,
    pub members: Vec<MemberDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    pub is_abstract: bool,
    pub is_sealed: bool,
    /// Declared with `actor` (Phase 10 mailbox isolation MVP).
    pub is_actor: bool,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub primary_ctor: Vec<PrimaryParam>,
    pub inheritance: Option<InheritanceClause>,
    pub where_clause: Vec<GenericConstraint>,
    pub members: Vec<MemberDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCase {
    pub span: Span,
    pub name: String,
    pub payload: Vec<(String, Type)>,
    pub raw_value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub inheritance: Option<InheritanceClause>,
    pub where_clause: Vec<GenericConstraint>,
    pub cases: Vec<EnumCase>,
    pub members: Vec<MemberDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub inheritance: Option<InheritanceClause>,
    pub where_clause: Vec<GenericConstraint>,
    pub members: Vec<MemberDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub extended_type: Type,
    pub generics: Vec<GenericParam>,
    pub inheritance: Option<InheritanceClause>,
    pub where_clause: Vec<GenericConstraint>,
    pub members: Vec<MemberDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberDecl {
    Func(FuncDecl),
    Var(VarDecl),
    Let(LetDecl),
    Init(InitDecl),
    Deinit(DeinitDecl),
    Nested(Box<Decl>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitDecl {
    pub span: Span,
    pub access: Option<Access>,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub where_clause: Vec<GenericConstraint>,
    pub throws: ThrowsClause,
    pub body: FuncBody,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeinitDecl {
    pub span: Span,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    pub is_async: bool,
    pub is_override: bool,
    pub is_virtual: bool,
    pub is_abstract: bool,
    pub is_sealed: bool,
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub where_clause: Vec<GenericConstraint>,
    pub throws: ThrowsClause,
    pub body: FuncBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FuncBody {
    Block(Block),
    Expr(Box<Expr>),
    Missing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThrowsClause {
    pub span: Span,
    pub types: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub span: Span,
    pub external_name: Option<String>,
    pub internal_name: String,
    pub ty: Type,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    /// `async let x = …` — structured concurrency binding (Phase 1.1b).
    pub is_async: bool,
    pub bindings: Vec<PatternBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyObserver {
    pub span: Span,
    pub param_name: Option<String>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComputedBody {
    /// `var x: T => expr`
    Arrow(Box<Expr>),
    /// `var x: T { get { ... } set { ... } }`
    Accessors {
        getter: Block,
        setter: Option<(Option<String>, Block)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub span: Span,
    pub attrs: Vec<Attribute>,
    pub access: Option<Access>,
    /// Narrower setter visibility from `private(set)` / `fileprivate(set)` / `internal(set)`.
    pub setter_access: Option<Access>,
    pub modifiers: ParamModifiers,
    pub bindings: Vec<PatternBinding>,
    /// Computed property body (`=>` or get/set). Mutually exclusive with stored init+observers.
    pub computed: Option<ComputedBody>,
    pub will_set: Option<PropertyObserver>,
    pub did_set: Option<PropertyObserver>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub span: Span,
    pub access: Option<Access>,
    pub name: String,
    pub ty: Option<Type>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternBinding {
    pub span: Span,
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub initializer: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named {
        span: Span,
        name: String,
        generics: Vec<Type>,
    },
    Optional {
        span: Span,
        inner: Box<Type>,
    },
    Tuple {
        span: Span,
        elements: Vec<(Option<String>, Type)>,
    },
    Function {
        span: Span,
        params: Vec<Type>,
        return_type: Box<Type>,
        throws: ThrowsClause,
    },
    Array {
        span: Span,
        element: Box<Type>,
    },
    Dictionary {
        span: Span,
        key: Box<Type>,
        value: Box<Type>,
    },
    Attributed {
        span: Span,
        attribute: String,
        inner: Box<Type>,
    },
    Error {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard { span: Span },
    Ident {
        span: Span,
        name: String,
    },
    Typed {
        span: Span,
        pattern: Box<Pattern>,
        ty: Type,
    },
    Tuple {
        span: Span,
        elements: Vec<Pattern>,
    },
    EnumCase {
        span: Span,
        type_name: Option<String>,
        case: String,
        subpatterns: Vec<Pattern>,
    },
    Literal {
        span: Span,
        expr: Box<Expr>,
    },
    Binding {
        span: Span,
        kind: StorageKind,
        pattern: Box<Pattern>,
    },
    Error {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    LiteralInt {
        span: Span,
        value: String,
    },
    LiteralFloat {
        span: Span,
        value: String,
    },
    LiteralString {
        span: Span,
        parts: Vec<StringPart>,
    },
    LiteralBool {
        span: Span,
        value: bool,
    },
    LiteralNil {
        span: Span,
    },
    LiteralChar {
        span: Span,
        value: char,
    },
    Ident {
        span: Span,
        name: String,
    },
    /// `$IDENT` compile-time constant (replaced with a literal before sema).
    CompileConst {
        span: Span,
        name: String,
    },
    SelfExpr {
        span: Span,
        kind: SelfKind,
    },
    Array {
        span: Span,
        elements: Vec<Expr>,
    },
    Dictionary {
        span: Span,
        entries: Vec<(Expr, Expr)>,
    },
    Unary {
        span: Span,
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        span: Span,
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        span: Span,
        callee: Box<Expr>,
        args: Vec<CallArg>,
        try_kind: TryCallKind,
    },
    Member {
        span: Span,
        base: Box<Expr>,
        name: String,
        optional: bool,
    },
    Index {
        span: Span,
        base: Box<Expr>,
        indices: Vec<Expr>,
        optional: bool,
    },
    Assign {
        span: Span,
        target: Box<Expr>,
        op: AssignOp,
        value: Box<Expr>,
    },
    If {
        span: Span,
        arms: Vec<IfArm>,
        else_branch: Option<IfElse>,
    },
    Switch {
        span: Span,
        subject: Box<Expr>,
        arms: Vec<SwitchArm>,
        default: Option<Box<Expr>>,
    },
    Closure {
        span: Span,
        params: Vec<ClosureParam>,
        return_type: Option<Type>,
        body: ClosureBody,
    },
    Lambda {
        span: Span,
        params: Vec<ClosureParam>,
        body: Box<Expr>,
    },
    Try {
        span: Span,
        expr: Box<Expr>,
        optional: bool,
        /// `try!` — trap on error.
        force: bool,
    },
    Await {
        span: Span,
        expr: Box<Expr>,
    },
    Cast {
        span: Span,
        expr: Box<Expr>,
        ty: Type,
        kind: CastKind,
    },
    /// `expr is Type` or `expr is Type name` (optional binding).
    Is {
        span: Span,
        expr: Box<Expr>,
        ty: Type,
        binding: Option<String>,
    },
    Ternary {
        span: Span,
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    NilCoalesce {
        span: Span,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Postfix force-unwrap `x!` (distinct from prefix `!` / `try!`).
    ForceUnwrap {
        span: Span,
        expr: Box<Expr>,
    },
    Range {
        span: Span,
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },
    DoCatch {
        span: Span,
        body: Block,
        catches: Vec<CatchClause>,
    },
    Catch {
        span: Span,
        expr: Box<Expr>,
        catches: Vec<CatchClause>,
    },
    Unsafe {
        span: Span,
        body: UnsafeBody,
    },
    Paren {
        span: Span,
        expr: Box<Expr>,
    },
    Error {
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfKind {
    Instance,
    Type,
    Super,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Text(String),
    Interpolation(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
    BitNot,
    Deref,
    Addr,
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
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryCallKind {
    None,
    Try,
    TryOptional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    As,
    AsOptional,
    AsForced,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallArg {
    pub span: Span,
    pub label: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfArm {
    pub span: Span,
    pub condition: IfCondition,
    pub body: IfBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IfCondition {
    Expr(Expr),
    Let(Pattern, Option<Expr>),
    Var(Pattern, Option<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IfBody {
    Block(Block),
    Arrow(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IfElse {
    Block(Block),
    Expr(Box<Expr>),
    If(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchArm {
    pub span: Span,
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: SwitchBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SwitchBody {
    Arrow(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureParam {
    pub span: Span,
    pub name: Option<String>,
    pub ty: Option<Type>,
    pub pattern: Option<Pattern>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClosureBody {
    Expr(Box<Expr>),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub span: Span,
    pub pattern: Option<Pattern>,
    pub body: CatchBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CatchBody {
    Block(Block),
    Arrow(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnsafeBody {
    Block(Block),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr {
        span: Span,
        expr: Expr,
    },
    Let(LetDecl),
    Var(VarDecl),
    Return {
        span: Span,
        value: Option<Expr>,
    },
    Throw {
        span: Span,
        value: Expr,
    },
    Break {
        span: Span,
        label: Option<String>,
    },
    Continue {
        span: Span,
        label: Option<String>,
    },
    Guard {
        span: Span,
        condition: GuardCondition,
        else_body: Block,
    },
    If {
        span: Span,
        arms: Vec<IfArm>,
        else_branch: Option<IfElse>,
    },
    While {
        span: Span,
        label: Option<String>,
        condition: Expr,
        body: Block,
    },
    For {
        span: Span,
        label: Option<String>,
        pattern: Pattern,
        sequence: Expr,
        filter: Option<Expr>,
        body: Block,
    },
    DoCatch {
        span: Span,
        body: Block,
        catches: Vec<CatchClause>,
    },
    Defer {
        span: Span,
        /// Expression form `#defer foo()` or block `#defer { ... }` / keyword `defer { ... }`.
        body: DeferBody,
    },
    /// `#die "message"` — abort immediately.
    Die {
        span: Span,
        message: String,
    },
    /// `#if $COND { ... } else { ... }` — dead branch dropped before sema.
    CompileIf {
        span: Span,
        condition: Expr,
        then_body: Block,
        else_body: Option<Block>,
    },
    /// `@run call()` — fire-and-forget task spawn (Phase 10).
    Run {
        span: Span,
        expr: Expr,
    },
    Block(Block),
    Error {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeferBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuardCondition {
    Expr(Expr),
    Let(Pattern, Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
}
