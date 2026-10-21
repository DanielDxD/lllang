//! LLLang lexer, parser, AST, and syntax diagnostics.

mod ast;
mod diagnostic;
mod fmt;
mod lexer;
mod parser;
mod span;
mod token;

pub use ast::*;
pub use diagnostic::{codes as diagnostic_codes, Diagnostic, Severity, SyntaxError};
pub use fmt::format_source;
pub use lexer::{lex, Lexer};
pub use span::{Loc, SourceMap, Span};
pub use token::{Token, TokenKind};

/// Result of parsing a compilation unit.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub file: File,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse LLLang source into an AST and diagnostics (lex + parse errors).
pub fn parse(source: &str) -> ParseResult {
    let (file, diagnostics) = parser::Parser::parse_source(source);
    ParseResult { file, diagnostics }
}

/// Syntax-check source; returns only error-severity diagnostics from parsing.
pub fn check(source: &str) -> Vec<Diagnostic> {
    parse(source)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}
