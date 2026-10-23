//! Lexical analysis for LLLang source.

use crate::diagnostic::{codes, Diagnostic, Severity};
use crate::span::Span;
use crate::token::{Token, TokenKind};

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = matches!(token.kind, TokenKind::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Token {
        self.skip_trivia();
        let start = self.pos;
        if start >= self.bytes.len() {
            return Token::new(TokenKind::Eof, Span::new(start as u32, start as u32));
        }

        let b = self.bytes[self.pos];
        let kind = match b {
            b'@' => {
                self.advance(1);
                TokenKind::At
            }
            b'(' => {
                self.advance(1);
                TokenKind::LParen
            }
            b')' => {
                self.advance(1);
                TokenKind::RParen
            }
            b'[' => {
                self.advance(1);
                TokenKind::LBracket
            }
            b']' => {
                self.advance(1);
                TokenKind::RBracket
            }
            b'{' => {
                self.advance(1);
                TokenKind::LBrace
            }
            b'}' => {
                self.advance(1);
                TokenKind::RBrace
            }
            b',' => {
                self.advance(1);
                TokenKind::Comma
            }
            b';' => {
                self.advance(1);
                TokenKind::Semicolon
            }
            b':' => {
                self.advance(1);
                TokenKind::Colon
            }
            b'.' => self.lex_dot(),
            b'+' => self.lex_plus(),
            b'-' => self.lex_minus(),
            b'*' => self.lex_star(),
            b'/' => self.lex_slash(),
            b'%' => self.lex_percent(),
            b'=' => self.lex_eq(),
            b'!' => self.lex_bang(),
            b'<' => self.lex_lt(),
            b'>' => self.lex_gt(),
            b'&' => self.lex_amp(),
            b'|' => self.lex_pipe(),
            b'^' => {
                self.advance(1);
                TokenKind::Caret
            }
            b'~' => {
                self.advance(1);
                TokenKind::Tilde
            }
            b'?' => self.lex_question(),
            b'#' => {
                self.advance(1);
                TokenKind::Hash
            }
            b'$' => self.lex_dollar_ident(),
            b'\'' => self.lex_char(),
            b'"' => self.lex_string(),
            b'0'..=b'9' => self.lex_number(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident_or_keyword(),
            _ if is_ident_start(b) => self.lex_ident_or_keyword(),
            _ => {
                self.error(
                    codes::UNEXPECTED_TOKEN,
                    format!("unexpected character `{chr}`", chr = b as char),
                    Span::new(start as u32, (start + 1) as u32),
                );
                self.advance(1);
                TokenKind::Error
            }
        };

        Token::new(kind, Span::new(start as u32, self.pos as u32))
    }

    fn lex_dot(&mut self) -> TokenKind {
        if self.starts_with("..<") {
            self.advance(3);
            TokenKind::DotDotLt
        } else if self.starts_with("...") {
            self.advance(3);
            TokenKind::DotDotDot
        } else {
            self.advance(1);
            TokenKind::Dot
        }
    }

    fn lex_plus(&mut self) -> TokenKind {
        if self.starts_with("+=") {
            self.advance(2);
            TokenKind::PlusEq
        } else {
            self.advance(1);
            TokenKind::Plus
        }
    }

    fn lex_minus(&mut self) -> TokenKind {
        if self.starts_with("->") {
            self.advance(2);
            TokenKind::Arrow
        } else if self.starts_with("-=") {
            self.advance(2);
            TokenKind::MinusEq
        } else {
            self.advance(1);
            TokenKind::Minus
        }
    }

    fn lex_star(&mut self) -> TokenKind {
        if self.starts_with("*=") {
            self.advance(2);
            TokenKind::StarEq
        } else {
            self.advance(1);
            TokenKind::Star
        }
    }

    fn lex_slash(&mut self) -> TokenKind {
        // Line/block comments are handled in `skip_trivia` before this runs.
        if self.starts_with("/=") {
            self.advance(2);
            TokenKind::SlashEq
        } else {
            self.advance(1);
            TokenKind::Slash
        }
    }

    fn lex_percent(&mut self) -> TokenKind {
        if self.starts_with("%=") {
            self.advance(2);
            TokenKind::PercentEq
        } else {
            self.advance(1);
            TokenKind::Percent
        }
    }

    fn skip_block_comment(&mut self) -> bool {
        let start = self.pos;
        self.advance(2);
        let mut depth = 1usize;
        while self.pos < self.bytes.len() {
            if self.starts_with("/*") {
                depth += 1;
                self.advance(2);
            } else if self.starts_with("*/") {
                depth -= 1;
                self.advance(2);
                if depth == 0 {
                    return true;
                }
            } else {
                self.advance(1);
            }
        }
        self.error(
            codes::UNTERMINATED_BLOCK_COMMENT,
            "unterminated block comment",
            Span::new(start as u32, self.pos as u32),
        );
        false
    }

    fn lex_eq(&mut self) -> TokenKind {
        if self.starts_with("=>") {
            self.advance(2);
            TokenKind::FatArrow
        } else if self.starts_with("==") {
            self.advance(2);
            TokenKind::EqEq
        } else {
            self.advance(1);
            TokenKind::Assign
        }
    }

    fn lex_bang(&mut self) -> TokenKind {
        if self.starts_with("!=") {
            self.advance(2);
            TokenKind::NotEq
        } else {
            self.advance(1);
            TokenKind::Bang
        }
    }

    fn lex_lt(&mut self) -> TokenKind {
        if self.starts_with("<<=") {
            self.advance(3);
            return TokenKind::Assign;
        }
        if self.starts_with("<<") {
            self.advance(2);
            TokenKind::Shl
        } else if self.starts_with("<=") {
            self.advance(2);
            TokenKind::Le
        } else {
            self.advance(1);
            TokenKind::Lt
        }
    }

    fn lex_gt(&mut self) -> TokenKind {
        if self.starts_with(">>=") {
            self.advance(3);
            return TokenKind::Assign;
        }
        if self.starts_with(">>") {
            self.advance(2);
            TokenKind::Shr
        } else if self.starts_with(">=") {
            self.advance(2);
            TokenKind::Ge
        } else {
            self.advance(1);
            TokenKind::Gt
        }
    }

    fn lex_amp(&mut self) -> TokenKind {
        if self.starts_with("&&") {
            self.advance(2);
            TokenKind::AndAnd
        } else {
            self.advance(1);
            TokenKind::Amp
        }
    }

    fn lex_pipe(&mut self) -> TokenKind {
        if self.starts_with("||") {
            self.advance(2);
            TokenKind::OrOr
        } else {
            self.advance(1);
            TokenKind::Pipe
        }
    }

    fn lex_question(&mut self) -> TokenKind {
        if self.starts_with("?.") {
            self.advance(2);
            TokenKind::QuestionDot
        } else if self.starts_with("??") {
            self.advance(2);
            TokenKind::NilCoalesce
        } else {
            self.advance(1);
            TokenKind::Question
        }
    }

    fn lex_char(&mut self) -> TokenKind {
        let start = self.pos;
        self.advance(1);
        if self.pos >= self.bytes.len() {
            self.error(codes::UNTERMINATED_STRING, "unterminated character literal", Span::new(start as u32, self.pos as u32));
            return TokenKind::Error;
        }
        let ch = if self.bytes[self.pos] == b'\\' {
            match self.read_escape() {
                Some(c) => c,
                None => return TokenKind::Error,
            }
        } else {
            let c = self.source[self.pos..].chars().next().unwrap_or('\0');
            let len = c.len_utf8();
            self.advance(len);
            c
        };
        if self.peek_byte() != Some(b'\'') {
            self.error(codes::UNTERMINATED_STRING, "unterminated character literal", Span::new(start as u32, self.pos as u32));
            return TokenKind::Error;
        }
        self.advance(1);
        TokenKind::Character(ch)
    }

    fn lex_string(&mut self) -> TokenKind {
        let start = self.pos;
        self.advance(1);
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'"' {
                self.advance(1);
                return TokenKind::String(out);
            }
            if b == b'\\' {
                self.advance(1);
                if self.pos < self.bytes.len() && self.bytes[self.pos] == b'(' {
                    out.push_str("\\(");
                    self.advance(1);
                    continue;
                }
                if let Some(c) = self.read_escape_char() {
                    out.push(c);
                } else {
                    return TokenKind::Error;
                }
                continue;
            }
            if b == b'\n' || b == b'\r' {
                self.error(codes::UNTERMINATED_STRING, "unterminated string literal", Span::new(start as u32, self.pos as u32));
                return TokenKind::Error;
            }
            let c = self.source[self.pos..].chars().next().unwrap_or('\0');
            out.push(c);
            self.advance(c.len_utf8());
        }
        self.error(codes::UNTERMINATED_STRING, "unterminated string literal", Span::new(start as u32, self.pos as u32));
        TokenKind::Error
    }

    fn read_escape(&mut self) -> Option<char> {
        self.advance(1);
        self.read_escape_char()
    }

    fn read_escape_char(&mut self) -> Option<char> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        let b = self.bytes[self.pos];
        self.advance(1);
        match b {
            b'n' => Some('\n'),
            b'r' => Some('\r'),
            b't' => Some('\t'),
            b'0' => Some('\0'),
            b'\\' => Some('\\'),
            b'\'' => Some('\''),
            b'"' => Some('"'),
            b'u' => self.read_unicode_escape(),
            _ => {
                self.error(codes::UNTERMINATED_STRING, format!("invalid escape sequence `\\{esc}`", esc = b as char), Span::new(self.pos.saturating_sub(1) as u32, self.pos as u32));
                None
            }
        }
    }

    fn read_unicode_escape(&mut self) -> Option<char> {
        if self.peek_byte() != Some(b'{') {
            return None;
        }
        self.advance(1);
        let hex_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_hexdigit() {
            self.advance(1);
        }
        if self.peek_byte() != Some(b'}') {
            return None;
        }
        let hex = &self.source[hex_start..self.pos];
        self.advance(1);
        u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
    }

    fn lex_number(&mut self) -> TokenKind {
        let start = self.pos;
        if self.bytes[self.pos] == b'0' {
            if self.peek_byte_at(1) == Some(b'x') || self.peek_byte_at(1) == Some(b'X') {
                self.advance(2);
                self.consume_digits(|b| b.is_ascii_hexdigit());
                return TokenKind::Integer(self.slice(start));
            }
            if self.peek_byte_at(1) == Some(b'b') || self.peek_byte_at(1) == Some(b'B') {
                self.advance(2);
                self.consume_digits(|b| b == b'0' || b == b'1');
                return TokenKind::Integer(self.slice(start));
            }
            if self.peek_byte_at(1) == Some(b'o') || self.peek_byte_at(1) == Some(b'O') {
                self.advance(2);
                self.consume_digits(|b| (b'0'..=b'7').contains(&b));
                return TokenKind::Integer(self.slice(start));
            }
        }
        self.consume_digits(|b| b.is_ascii_digit());
        if self.peek_byte() == Some(b'.') && self.peek_byte_at(1).map(|b| b.is_ascii_digit()) == Some(true) {
            self.advance(1);
            self.consume_digits(|b| b.is_ascii_digit());
            if self.peek_byte() == Some(b'e') || self.peek_byte() == Some(b'E') {
                self.advance(1);
                if self.peek_byte() == Some(b'+') || self.peek_byte() == Some(b'-') {
                    self.advance(1);
                }
                self.consume_digits(|b| b.is_ascii_digit());
            }
            return TokenKind::Float(self.slice(start));
        }
        if self.peek_byte() == Some(b'e') || self.peek_byte() == Some(b'E') {
            self.advance(1);
            if self.peek_byte() == Some(b'+') || self.peek_byte() == Some(b'-') {
                self.advance(1);
            }
            self.consume_digits(|b| b.is_ascii_digit());
            return TokenKind::Float(self.slice(start));
        }
        TokenKind::Integer(self.slice(start))
    }

    fn lex_dollar_ident(&mut self) -> TokenKind {
        let start = self.pos;
        self.advance(1); // `$`
        let Some(b) = self.peek_byte() else {
            self.error(
                codes::UNEXPECTED_TOKEN,
                "expected identifier after `$`",
                Span::new(start as u32, self.pos as u32),
            );
            return TokenKind::Error;
        };
        if !(b.is_ascii_alphabetic() || b == b'_' || is_ident_start(b)) {
            self.error(
                codes::UNEXPECTED_TOKEN,
                "expected identifier after `$`",
                Span::new(start as u32, self.pos as u32),
            );
            return TokenKind::Error;
        }
        let name_start = self.pos;
        self.advance(1);
        while let Some(b) = self.peek_byte() {
            if b.is_ascii_alphanumeric() || b == b'_' || is_ident_continue(b) {
                self.advance(1);
            } else {
                break;
            }
        }
        let name = self.source[name_start..self.pos].to_string();
        TokenKind::DollarIdent(name)
    }

    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if is_ident_continue(b) {
                self.advance(1);
            } else {
                break;
            }
        }
        let text = self.slice(start);
        keyword_kind(&text).unwrap_or(TokenKind::Ident(text))
    }

    fn skip_trivia(&mut self) {
        loop {
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.advance(1);
            }
            if self.starts_with("//") {
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.advance(1);
                }
            } else if self.starts_with("/*") {
                let _ = self.skip_block_comment();
            } else {
                break;
            }
        }
    }

    fn consume_digits<F: Fn(u8) -> bool>(&mut self, pred: F) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'_' {
                self.advance(1);
                continue;
            }
            if pred(b) {
                self.advance(1);
            } else {
                break;
            }
        }
    }

    fn slice(&self, start: usize) -> String {
        self.source[start..self.pos].to_string()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.source[self.pos..].starts_with(s)
    }

    fn peek_byte(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_byte_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.bytes.len());
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic {
            code: code.to_string(),
            message: message.into(),
            span,
            severity: Severity::Error,
            file: None,
        });
    }
}

fn is_ident_start(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

fn is_ident_continue(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

fn keyword_kind(text: &str) -> Option<TokenKind> {
    Some(match text {
        "namespace" => TokenKind::Namespace,
        "import" => TokenKind::Import,
        "struct" => TokenKind::Struct,
        "class" => TokenKind::Class,
        "actor" => TokenKind::Actor,
        "enum" => TokenKind::Enum,
        "protocol" => TokenKind::Protocol,
        "extension" => TokenKind::Extension,
        "func" => TokenKind::Func,
        "let" => TokenKind::Let,
        "var" => TokenKind::Var,
        "const" => TokenKind::Const,
        "typealias" => TokenKind::Typealias,
        "init" => TokenKind::Init,
        "deinit" => TokenKind::Deinit,
        "subscript" => TokenKind::Subscript,
        "operator" => TokenKind::Operator,
        "async" => TokenKind::Async,
        "await" => TokenKind::Await,
        "unsafe" => TokenKind::Unsafe,
        "open" => TokenKind::Open,
        "public" => TokenKind::Public,
        "internal" => TokenKind::Internal,
        "private" => TokenKind::Private,
        "protected" => TokenKind::Protected,
        "fileprivate" => TokenKind::Fileprivate,
        "static" => TokenKind::Static,
        "override" => TokenKind::Override,
        "virtual" => TokenKind::Virtual,
        "abstract" => TokenKind::Abstract,
        "sealed" => TokenKind::Sealed,
        "inout" => TokenKind::Inout,
        "weak" => TokenKind::Weak,
        "unowned" => TokenKind::Unowned,
        "mut" => TokenKind::Mut,
        "get" => TokenKind::Get,
        "set" => TokenKind::Set,
        "didSet" => TokenKind::DidSet,
        "willSet" => TokenKind::WillSet,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "elseif" => TokenKind::Elseif,
        "switch" => TokenKind::Switch,
        "default" => TokenKind::Default,
        "where" => TokenKind::Where,
        "guard" => TokenKind::Guard,
        "while" => TokenKind::While,
        "repeat" => TokenKind::Repeat,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "return" => TokenKind::Return,
        "throw" => TokenKind::Throw,
        "throws" => TokenKind::Throws,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "do" => TokenKind::Do,
        "defer" => TokenKind::Defer,
        "fallthrough" => TokenKind::Fallthrough,
        "self" => TokenKind::SelfLower,
        "Self" => TokenKind::SelfUpper,
        "super" => TokenKind::Super,
        "nil" => TokenKind::Nil,
        "is" => TokenKind::Is,
        "as" => TokenKind::As,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "true" => TokenKind::Bool(true),
        "false" => TokenKind::Bool(false),
        _ => return None,
    })
}

pub fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let diags = lexer.take_diagnostics();
    (tokens, diags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source).0.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn hello_program_tokens() {
        let source = r#"namespace Hello
import Std.IO
public func main() {
    print("Hello, LLLang!")
}"#;
        let (tokens, diags) = lex(source);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Namespace)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Func)));
        assert!(matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Eof)));
    }

    #[test]
    fn guard_line_tokens() {
        let source = r#"guard let d = delegate else { return }"#;
        let (tokens, diags) = lex(source);
        assert!(diags.is_empty());
        let kinds: Vec<_> = tokens.iter().map(|t| (&t.kind, t.span)).collect();
        assert!(
            kinds.iter().any(|(k, _)| matches!(k, TokenKind::Assign)),
            "{kinds:?}"
        );
    }

    #[test]
    fn operators_and_keywords() {
        let k = kinds("&& || ! ?? ?. => ->");
        assert!(k.contains(&TokenKind::AndAnd));
        assert!(k.contains(&TokenKind::FatArrow));
    }

    #[test]
    fn lexes_plus_eq() {
        let k = kinds("n += 2");
        assert!(k.contains(&TokenKind::PlusEq), "{k:?}");
    }
}
