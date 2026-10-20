//! Built-in types, protocols, and functions injected for every compilation unit.

use crate::types::Ty;
use indexmap::IndexMap;

pub const KNOWN_IMPORTS: &[&str] = &[
    "Std.IO",
    "Std.Core",
    "Std.PackageDescription",
    "Std.SolutionDescription",
];

pub fn is_known_import(path: &str) -> bool {
    KNOWN_IMPORTS.contains(&path)
}

/// Built-in free functions by simple name.
pub fn builtin_funcs() -> IndexMap<String, (Vec<Ty>, Ty, Option<Ty>, &'static str)> {
    let mut m = IndexMap::new();
    m.insert(
        "print".into(),
        (vec![Ty::String], Ty::Void, None, "print"),
    );
    m.insert(
        "int_from_string".into(),
        (vec![Ty::String], Ty::optional(Ty::Int), None, "int_from_string"),
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
    m
}

/// Instance members/properties by type name.
pub fn builtin_members() -> IndexMap<(String, String), (Ty, bool, &'static str)> {
    let mut m = IndexMap::new();
    // (type, member) -> (ty, is_property, builtin_id)
    m.insert(
        ("String".into(), "isEmpty".into()),
        (Ty::Bool, true, "string_is_empty"),
    );
    m
}

pub fn prelude_protocols() -> Vec<&'static str> {
    vec!["Error", "Equatable"]
}
