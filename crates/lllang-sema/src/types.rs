//! Semantic types for LLLang (Phase 2).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
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
    Optional(Box<Ty>),
    Array(Box<Ty>),
    Tuple(Vec<Ty>),
    Named(String),
    /// `Result<T, E>` with typed payloads (not Int-erased).
    Result {
        ok: Box<Ty>,
        err: Box<Ty>,
    },
    Func {
        params: Vec<Ty>,
        ret: Box<Ty>,
        throws: Option<Box<Ty>>,
    },
    Protocol(String),
    Class(String),
    Struct(String),
    Enum(String),
    Error,
}

impl Ty {
    pub fn is_class_like(&self) -> bool {
        matches!(self, Ty::Class(_) | Ty::Protocol(_))
    }

    pub fn display(&self) -> String {
        match self {
            Ty::Void => "Void".into(),
            Ty::Never => "Never".into(),
            Ty::Bool => "Bool".into(),
            Ty::Int => "Int".into(),
            Ty::Int8 => "Int8".into(),
            Ty::Int16 => "Int16".into(),
            Ty::Int32 => "Int32".into(),
            Ty::Int64 => "Int64".into(),
            Ty::UInt => "UInt".into(),
            Ty::Float => "Float".into(),
            Ty::Double => "Double".into(),
            Ty::Char => "Char".into(),
            Ty::String => "String".into(),
            Ty::Optional(inner) => format!("{}?", inner.display()),
            Ty::Array(inner) => format!("[{}]", inner.display()),
            Ty::Tuple(elems) => {
                let parts: Vec<_> = elems.iter().map(|t| t.display()).collect();
                format!("({})", parts.join(", "))
            }
            Ty::Named(name) => name.clone(),
            Ty::Result { ok, err } => format!("Result<{}, {}>", ok.display(), err.display()),
            Ty::Func { params, ret, throws } => {
                let ps: Vec<_> = params.iter().map(|t| t.display()).collect();
                let mut s = format!("({}) -> {}", ps.join(", "), ret.display());
                if let Some(e) = throws {
                    s.push_str(&format!(" throws({})", e.display()));
                }
                s
            }
            Ty::Protocol(name) => name.clone(),
            Ty::Class(name) => name.clone(),
            Ty::Struct(name) => name.clone(),
            Ty::Enum(name) => name.clone(),
            Ty::Error => "<error>".into(),
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Ty::Error)
    }

    /// Phase-2 assignability (pragmatic, not full Swift subtyping).
    pub fn assignable_from(&self, found: &Ty) -> bool {
        if self.is_error() || found.is_error() {
            return true;
        }
        if self == found {
            return true;
        }
        match (self, found) {
            (Ty::Optional(exp), Ty::Optional(got)) => exp.assignable_from(got),
            (Ty::Optional(exp), _) if found == &Ty::Void => false,
            (Ty::Optional(exp), got) => exp.assignable_from(got),
            (Ty::Double, Ty::Float) => true,
            (Ty::Float, Ty::Double) => true,
            (Ty::Named(a), Ty::Named(b)) if a == b => true,
            (Ty::Named(n), _) if n == "Any" => true,
            // Allow Nil placeholder for missing labeled args to soft-fail.
            (_, Ty::Error) => true,
            (Ty::Struct(a), Ty::Struct(b)) if a == b => true,
            (Ty::Class(a), Ty::Class(b)) if a == b => true,
            (Ty::Enum(a), Ty::Enum(b)) if a == b => true,
            (Ty::Result { ok: a, err: e }, Ty::Result { ok: b, err: f }) => {
                a.assignable_from(b) && e.assignable_from(f)
            }
            (Ty::Enum(n), Ty::Result { .. }) | (Ty::Result { .. }, Ty::Enum(n)) if n == "Result" => {
                true
            }
            // Codable / Encodable / Decodable accept any concrete struct/class.
            (Ty::Named(n), Ty::Struct(_) | Ty::Class(_))
                if n == "Encodable" || n == "Decodable" || n == "Codable" =>
            {
                true
            }
            (Ty::Protocol(n), Ty::Struct(_) | Ty::Class(_))
                if n == "Encodable" || n == "Decodable" || n == "Codable" || n == "Equatable" =>
            {
                true
            }
            // Provider protocols / aliases.
            (Ty::Named(n) | Ty::Struct(n), Ty::Struct(m) | Ty::Class(m) | Ty::Named(m))
                if n == "ChatModel"
                    && (m == "OllamaProvider"
                        || m == "OpenAIProvider"
                        || m == "AnthropicProvider"
                        || m == "ChatModel") =>
            {
                true
            }
            _ => false,
        }
    }

    pub fn optional(inner: Ty) -> Ty {
        Ty::Optional(Box::new(inner))
    }
}
