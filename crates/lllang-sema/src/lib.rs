//! Semantic analysis for LLLang — name resolution and type checking (Phase 2).

mod check;
pub mod hir;
pub mod prelude;
pub mod types;

pub use types::Ty;
pub use lllang_syntax::Diagnostic;

pub struct SemaResult {
    pub module: hir::Module,
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse each source file, collect syntax diagnostics, then name-resolve and type-check.
pub fn analyze(sources: &[(String, String)]) -> SemaResult {
    check::analyze_sources(sources)
}
