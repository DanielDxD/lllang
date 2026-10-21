//! Parse and lex diagnostics.

use crate::span::Span;

/// Error wrapper for tooling that prefers `thiserror` over raw diagnostics.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}")]
pub struct SyntaxError {
    pub code: String,
    pub message: String,
    pub span: Span,
}

impl From<Diagnostic> for SyntaxError {
    fn from(d: Diagnostic) -> Self {
        Self {
            code: d.code,
            message: d.message,
            span: d.span,
        }
    }
}

/// Diagnostic severity for filtering and LSP mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// A single diagnostic with a stable `LL0xxx` code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub span: Span,
    pub severity: Severity,
    /// Source path when known (multi-file analysis / CLI rendering).
    pub file: Option<String>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span,
            severity: Severity::Error,
            file: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span,
            severity: Severity::Warning,
            file: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }
}

/// Stable diagnostic codes (Phase 0 — syntax).
pub mod codes {
    pub const INVALID_UTF8: &str = "LL0001";
    pub const UNTERMINATED_BLOCK_COMMENT: &str = "LL0002";
    pub const UNTERMINATED_STRING: &str = "LL0003";
    pub const INVALID_NUMERIC: &str = "LL0008";

    pub const UNEXPECTED_TOKEN: &str = "LL0100";
    pub const UNEXPECTED_EOF: &str = "LL0101";
    pub const EXPECTED_TYPE: &str = "LL0200";
    pub const UNCLOSED_GENERIC: &str = "LL0201";
    pub const EXPECTED_TYPE_BODY: &str = "LL0300";
}
