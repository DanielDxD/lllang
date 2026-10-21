//! Built-in types, protocols, and functions injected for every compilation unit.

use crate::types::Ty;
use indexmap::IndexMap;

pub const KNOWN_IMPORTS: &[&str] = &[
    "Std.IO",
    "Std.Core",
    "Std.Json",
    "Std.Toon",
    "Std.Http",
    "Std.Async",
    "Std.Llm",
    "Std.PackageDescription",
    "Std.SolutionDescription",
    "Std.Testing",
];

pub fn is_known_import(path: &str) -> bool {
    KNOWN_IMPORTS.contains(&path) || path.starts_with("Std.")
}

/// Built-in free functions by simple name.
pub fn builtin_funcs() -> IndexMap<String, (Vec<Ty>, Ty, Option<Ty>, &'static str)> {
    let mut m = IndexMap::new();
    m.insert(
        "print".into(),
        (vec![Ty::String], Ty::Void, None, "print"),
    );
    m.insert(
        "__test_expect".into(),
        (vec![Ty::Bool, Ty::String], Ty::Void, None, "test_expect"),
    );
    m.insert(
        "__test_expect_empty".into(),
        (vec![Ty::String, Ty::String], Ty::Void, None, "test_expect_empty"),
    );
    m.insert(
        "__test_expect_not_empty".into(),
        (vec![Ty::String, Ty::String], Ty::Void, None, "test_expect_not_empty"),
    );
    m.insert(
        "__test_contains".into(),
        (
            vec![Ty::String, Ty::String, Ty::String],
            Ty::Void,
            None,
            "test_contains",
        ),
    );
    m.insert(
        "__test_approx".into(),
        (
            vec![Ty::Double, Ty::Double, Ty::String],
            Ty::Void,
            None,
            "test_approx",
        ),
    );
    m.insert(
        "__test_job_id".into(),
        (vec![], Ty::Int, None, "test_job_id"),
    );
    m.insert(
        "expect".into(),
        (vec![Ty::Bool], Ty::Void, None, "test_expect_bare"),
    );
    m.insert(
        "int_from_string".into(),
        (vec![Ty::String], Ty::optional(Ty::Int), None, "int_from_string"),
    );
    m.insert(
        "runAgent".into(),
        (
            vec![
                Ty::Named("ChatModel".into()),
                Ty::Named("ToolRegistry".into()),
                Ty::String,
            ],
            Ty::String,
            Some(Ty::Named("LlmError".into())),
            "llm_run_agent",
        ),
    );
    m
}

/// Static methods on type names (e.g. `Int.from`).
pub fn builtin_type_methods() -> IndexMap<(String, String), (Vec<Ty>, Ty, Option<Ty>, &'static str)> {
    let mut m = IndexMap::new();
    m.insert(
        ("Int".into(), "from".into()),
        (vec![Ty::String], Ty::optional(Ty::Int), None, "int_from_string"),
    );
    // JSON/TOON encode/decode — return type filled from expected annotation at call site.
    m.insert(
        ("JSON".into(), "decode".into()),
        (
            vec![Ty::String],
            Ty::Named("Decodable".into()),
            Some(Ty::Named("DecodingError".into())),
            "json_decode",
        ),
    );
    m.insert(
        ("JSON".into(), "encode".into()),
        (vec![Ty::Named("Encodable".into())], Ty::String, None, "json_encode"),
    );
    m.insert(
        ("TOON".into(), "decode".into()),
        (
            vec![Ty::String],
            Ty::Named("Decodable".into()),
            Some(Ty::Named("DecodingError".into())),
            "toon_decode",
        ),
    );
    m.insert(
        ("TOON".into(), "encode".into()),
        (vec![Ty::Named("Encodable".into())], Ty::String, None, "toon_encode"),
    );
    m.insert(
        ("Http".into(), "get".into()),
        (
            vec![Ty::String],
            Ty::String,
            Some(Ty::Named("HttpError".into())),
            "http_get",
        ),
    );
    m.insert(
        ("LlmContext".into(), "mergePackageContext".into()),
        (
            vec![Ty::String],
            Ty::Named("LlmContext".into()),
            Some(Ty::Named("LlmError".into())),
            "llm_merge_context",
        ),
    );
    m
}

/// Instance members/properties by type name.
/// Value: (result_ty, is_property, builtin_id)
pub fn builtin_members() -> IndexMap<(String, String), (Ty, bool, &'static str)> {
    let mut m = IndexMap::new();
    // (type, member) -> (ty, is_property, builtin_id)
    m.insert(
        ("String".into(), "isEmpty".into()),
        (Ty::Bool, true, "string_is_empty"),
    );
    m.insert(
        ("Double".into(), "squareRoot".into()),
        (Ty::Double, false, "double_sqrt"),
    );
    m
}

pub fn prelude_protocols() -> Vec<&'static str> {
    vec![
        "Error",
        "Equatable",
        "Codable",
        "Encodable",
        "Decodable",
    ]
}

/// Error types injected for stdlib MVP.
pub fn prelude_error_types() -> Vec<&'static str> {
    vec!["DecodingError", "EncodingError", "HttpError", "LlmError"]
}
