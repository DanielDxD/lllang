//! LLLang Language Server (Phase 1 — syntax-aware).
//!
//! Capabilities: diagnostics, document symbols, folding ranges,
//! semantic tokens, selection ranges, keyword/same-file hover & completion.

mod analysis;
mod semantic;
mod server;

pub use server::{run_stdio, Backend};

pub fn server_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
