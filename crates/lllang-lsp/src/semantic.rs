//! Semantic token legend and extraction (Phase 1 — keyword/ident heuristics).

use lllang_syntax::{lex, TokenKind};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

pub fn legend_token_types() -> Vec<SemanticTokenType> {
    vec![
        SemanticTokenType::KEYWORD,
        SemanticTokenType::TYPE,
        SemanticTokenType::FUNCTION,
        SemanticTokenType::VARIABLE,
        SemanticTokenType::PROPERTY,
        SemanticTokenType::NUMBER,
        SemanticTokenType::STRING,
        SemanticTokenType::COMMENT,
        SemanticTokenType::OPERATOR,
        SemanticTokenType::NAMESPACE,
        SemanticTokenType::ENUM_MEMBER,
        SemanticTokenType::PARAMETER,
    ]
}

fn token_type_index(kind: &TokenKind) -> Option<u32> {
    match kind {
        TokenKind::Integer(_) | TokenKind::Float(_) => Some(5),
        TokenKind::String(_) | TokenKind::Character(_) => Some(6),
        TokenKind::Ident(_) => Some(3),
        TokenKind::Bool(_) | TokenKind::Nil => Some(0),
        TokenKind::Eof | TokenKind::Error => None,
        // Keywords / modifiers / control
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
        | TokenKind::Typealias
        | TokenKind::Init
        | TokenKind::Deinit
        | TokenKind::Subscript
        | TokenKind::Operator
        | TokenKind::Async
        | TokenKind::Await
        | TokenKind::Unsafe
        | TokenKind::Public
        | TokenKind::Internal
        | TokenKind::Private
        | TokenKind::Protected
        | TokenKind::Fileprivate
        | TokenKind::Static
        | TokenKind::Override
        | TokenKind::Virtual
        | TokenKind::Abstract
        | TokenKind::Sealed
        | TokenKind::Inout
        | TokenKind::Weak
        | TokenKind::Unowned
        | TokenKind::Mut
        | TokenKind::Get
        | TokenKind::Set
        | TokenKind::DidSet
        | TokenKind::WillSet
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::Elseif
        | TokenKind::Switch
        | TokenKind::Default
        | TokenKind::Where
        | TokenKind::Guard
        | TokenKind::While
        | TokenKind::Repeat
        | TokenKind::For
        | TokenKind::In
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Return
        | TokenKind::Throw
        | TokenKind::Throws
        | TokenKind::Try
        | TokenKind::Catch
        | TokenKind::Do
        | TokenKind::Defer
        | TokenKind::Fallthrough
        | TokenKind::SelfLower
        | TokenKind::SelfUpper
        | TokenKind::Super
        | TokenKind::Is
        | TokenKind::As
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not => Some(0),
        // Operators / punctuation
        _ => Some(8),
    }
}

/// Encode semantic tokens as LSP deltas from a full document lex.
pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let (tokens, _) = lex(source);
    let mut out = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_col = 0u32;
    let map = lllang_syntax::SourceMap::new(source);

    for tok in tokens {
        if matches!(tok.kind, TokenKind::Eof) {
            break;
        }
        let Some(token_type) = token_type_index(&tok.kind) else {
            continue;
        };
        let loc = map.loc(tok.span.start);
        let line = loc.line.saturating_sub(1);
        let col = loc.column.saturating_sub(1);
        let length = tok.span.end.saturating_sub(tok.span.start);

        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            col
        };
        out.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });
        prev_line = line;
        prev_col = col;
    }
    out
}
