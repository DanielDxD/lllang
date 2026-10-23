//! Lexical tokens.

use crate::span::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn dummy(kind: TokenKind) -> Self {
        Self::new(kind, Span::EMPTY)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Literals
    Integer(String),
    Float(String),
    String(String),
    Character(char),
    Bool(bool),

    // Identifiers
    Ident(String),

    // Keywords — declarations
    Namespace,
    Import,
    Struct,
    Class,
    /// Phase 10: serial-mailbox reference type (parsed like `class`).
    Actor,
    Enum,
    Protocol,
    Extension,
    Func,
    Let,
    Var,
    Const,
    Typealias,
    Init,
    Deinit,
    Subscript,
    Operator,
    Async,
    Await,
    Unsafe,

    // Access
    Open,
    Public,
    Internal,
    Private,
    Protected,
    Fileprivate,

    // Modifiers
    Static,
    Override,
    Virtual,
    Abstract,
    Sealed,
    Inout,
    Weak,
    Unowned,
    Mut,
    Get,
    Set,
    DidSet,
    WillSet,

    // Control flow
    If,
    Else,
    Elseif,
    Switch,
    Default,
    Where,
    Guard,
    While,
    Repeat,
    For,
    In,
    Break,
    Continue,
    Return,
    Throw,
    Throws,
    Try,
    Catch,
    Do,
    Defer,
    Fallthrough,

    // Special names
    SelfLower,
    SelfUpper,
    Super,
    Nil,
    Is,
    As,

    // Logic keywords (aliases)
    And,
    Or,
    Not,

    // Punctuation
    At,
    /// `#` start of a compile-time / scope directive (`#defer`, `#die`, …).
    Hash,
    /// `$IDENT` compile-time constant (`$OS`, `$DEBUG`, …).
    DollarIdent(String),
    Dot,
    Comma,
    Colon,
    Semicolon,
    Question,
    Bang,
    Arrow,
    FatArrow,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Amp,
    Pipe,
    Caret,
    Tilde,
    EqEq,
    NotEq,
    Lt,
    Gt,
    Le,
    Ge,
    AndAnd,
    OrOr,
    QuestionDot,
    NilCoalesce,
    DotDotDot,
    DotDotLt,
    Shl,
    Shr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    // Synthetic
    Eof,
    Error,
}

impl TokenKind {
    pub fn is_top_level_start(&self) -> bool {
        matches!(
            self,
            TokenKind::Namespace
                | TokenKind::Import
                | TokenKind::Struct
                | TokenKind::Class
                | TokenKind::Enum
                | TokenKind::Protocol
                | TokenKind::Extension
                | TokenKind::Func
                | TokenKind::Let
                | TokenKind::Var
                | TokenKind::Const
                | TokenKind::At
                | TokenKind::Open
                | TokenKind::Public
                | TokenKind::Internal
                | TokenKind::Private
                | TokenKind::Protected
                | TokenKind::Fileprivate
                | TokenKind::Async
                | TokenKind::Unsafe
        )
    }
}
