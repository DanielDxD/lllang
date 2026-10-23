//! Built-in types, protocols, and functions injected for every compilation unit.

use crate::types::Ty;
use indexmap::IndexMap;

pub const KNOWN_IMPORTS: &[&str] = &[
    "Std.IO",
    "Std.Core",
    "Std.Text",
    "Std.Collections",
    "Std.Json",
    "Std.Toon",
    "Std.Http",
    "Std.Net",
    "Std.Log",
    "Std.CLI",
    "Std.Sync",
    "Std.Crypto",
    "Std.Async",
    "Std.SQL",
    "Std.Llm",
    "Std.FS",
    "Std.Env",
    "Std.Identity",
    "Std.Process",
    "Std.Time",
    "Std.Math",
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
        "eprint".into(),
        (vec![Ty::String], Ty::Void, None, "eprint"),
    );
    m.insert(
        "readLine".into(),
        (vec![], Ty::optional(Ty::String), None, "read_line"),
    );
    m.insert(
        "fatalError".into(),
        (vec![Ty::String], Ty::Never, None, "fatal_error"),
    );
    m.insert(
        "assert".into(),
        (vec![Ty::Bool, Ty::String], Ty::Void, None, "assert"),
    );
    m.insert(
        "precondition".into(),
        (vec![Ty::Bool, Ty::String], Ty::Void, None, "assert"),
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
    // Math free functions (Phase 1.1c)
    m.insert("abs".into(), (vec![Ty::Int], Ty::Int, None, "math_abs_i64"));
    m.insert(
        "max".into(),
        (vec![Ty::Int, Ty::Int], Ty::Int, None, "math_max_i64"),
    );
    m.insert(
        "min".into(),
        (vec![Ty::Int, Ty::Int], Ty::Int, None, "math_min_i64"),
    );
    m.insert(
        "clamp".into(),
        (vec![Ty::Int, Ty::Int, Ty::Int], Ty::Int, None, "math_clamp_i64"),
    );
    m.insert(
        "sqrt".into(),
        (vec![Ty::Double], Ty::Double, None, "math_sqrt"),
    );
    m.insert(
        "pow".into(),
        (vec![Ty::Double, Ty::Double], Ty::Double, None, "math_pow"),
    );
    m.insert("sin".into(), (vec![Ty::Double], Ty::Double, None, "math_sin"));
    m.insert(
        "log10".into(),
        (vec![Ty::Double], Ty::Double, None, "math_log10"),
    );
    m.insert("PI".into(), (vec![], Ty::Double, None, "math_pi"));
    m
}

/// Static methods on type names (e.g. `Int.from`).
pub fn builtin_type_methods() -> IndexMap<(String, String), (Vec<Ty>, Ty, Option<Ty>, &'static str)> {
    let mut m = IndexMap::new();
    m.insert(
        ("Int".into(), "from".into()),
        (vec![Ty::String], Ty::optional(Ty::Int), None, "int_from_string"),
    );
    m.insert(
        ("Environment".into(), "get".into()),
        (vec![Ty::String], Ty::optional(Ty::String), None, "env_get"),
    );
    m.insert(
        ("Env".into(), "get".into()),
        (vec![Ty::String], Ty::optional(Ty::String), None, "env_get"),
    );
    m.insert(
        ("Env".into(), "loadDotEnv".into()),
        (vec![Ty::String], Ty::Void, None, "env_load_dotenv"),
    );
    m.insert(
        ("Env".into(), "load".into()),
        (vec![Ty::String], Ty::Void, None, "env_load_dotenv"),
    );
    m.insert(
        ("Environment".into(), "loadDotEnv".into()),
        (vec![Ty::String], Ty::Void, None, "env_load_dotenv"),
    );
    m.insert(
        ("Environment".into(), "load".into()),
        (vec![Ty::String], Ty::Void, None, "env_load_dotenv"),
    );
    // Typed Env getters with default (arity-1 optional overloads resolved in check.rs).
    m.insert(
        ("Env".into(), "string".into()),
        (vec![Ty::String, Ty::String], Ty::String, None, "env_string_or"),
    );
    m.insert(
        ("Env".into(), "integer".into()),
        (vec![Ty::String, Ty::Int], Ty::Int, None, "env_get_i64_or"),
    );
    m.insert(
        ("Env".into(), "int".into()),
        (vec![Ty::String, Ty::Int], Ty::Int, None, "env_get_i64_or"),
    );
    m.insert(
        ("Env".into(), "bool".into()),
        (vec![Ty::String, Ty::Bool], Ty::Bool, None, "env_get_bool_or"),
    );
    m.insert(
        ("Env".into(), "float".into()),
        (vec![Ty::String, Ty::Double], Ty::Double, None, "env_get_f64_or"),
    );
    m.insert(
        ("Env".into(), "double".into()),
        (vec![Ty::String, Ty::Double], Ty::Double, None, "env_get_f64_or"),
    );
    m.insert(
        ("Env".into(), "decimal".into()),
        (
            vec![Ty::String, Ty::Named("Decimal".into())],
            Ty::Named("Decimal".into()),
            None,
            "env_get_decimal_or",
        ),
    );
    m.insert(
        ("Env".into(), "uint".into()),
        (vec![Ty::String, Ty::UInt], Ty::UInt, None, "env_get_i64_or"),
    );
    m.insert(
        ("Env".into(), "int64".into()),
        (vec![Ty::String, Ty::Int64], Ty::Int64, None, "env_get_i64_or"),
    );
    m.insert(
        ("Identity".into(), "uuid".into()),
        (vec![], Ty::String, None, "identity_uuid"),
    );
    m.insert(
        ("Identity".into(), "ulid".into()),
        (vec![], Ty::String, None, "identity_ulid"),
    );
    m.insert(
        ("Identity".into(), "nanoid".into()),
        (vec![], Ty::String, None, "identity_nanoid"),
    );
    m.insert(
        ("Decimal".into(), "from".into()),
        (
            vec![Ty::String],
            Ty::Named("Decimal".into()),
            None,
            "decimal_from_string",
        ),
    );
    m.insert(
        ("Decimal".into(), "add".into()),
        (
            vec![Ty::Named("Decimal".into()), Ty::Named("Decimal".into())],
            Ty::Named("Decimal".into()),
            None,
            "decimal_add",
        ),
    );
    m.insert(
        ("Decimal".into(), "sub".into()),
        (
            vec![Ty::Named("Decimal".into()), Ty::Named("Decimal".into())],
            Ty::Named("Decimal".into()),
            None,
            "decimal_sub",
        ),
    );
    m.insert(
        ("Decimal".into(), "mul".into()),
        (
            vec![Ty::Named("Decimal".into()), Ty::Named("Decimal".into())],
            Ty::Named("Decimal".into()),
            None,
            "decimal_mul",
        ),
    );
    m.insert(
        ("Base64".into(), "encode".into()),
        (vec![Ty::String], Ty::String, None, "base64_encode"),
    );
    m.insert(
        ("Hex".into(), "encode".into()),
        (vec![Ty::String], Ty::String, None, "hex_encode"),
    );
    m.insert(
        ("SecureRandom".into(), "bytes".into()),
        (vec![Ty::Int], Ty::String, None, "secure_random_bytes"),
    );
    m.insert(
        ("Process".into(), "start".into()),
        (vec![Ty::String], Ty::Named("ProcessHandle".into()), None, "process_start"),
    );
    m.insert(
        ("Process".into(), "wait".into()),
        (vec![Ty::Named("ProcessHandle".into())], Ty::Int, None, "process_wait"),
    );
    m.insert(
        ("Args".into(), "count".into()),
        (vec![], Ty::Int, None, "args_count"),
    );
    m.insert(
        ("Args".into(), "get".into()),
        (vec![Ty::Int], Ty::optional(Ty::String), None, "args_get"),
    );
    m.insert(
        ("File".into(), "exists".into()),
        (vec![Ty::String], Ty::Bool, None, "file_exists"),
    );
    m.insert(
        ("File".into(), "readText".into()),
        (vec![Ty::String], Ty::String, None, "file_read_text"),
    );
    m.insert(
        ("File".into(), "writeText".into()),
        (vec![Ty::String, Ty::String], Ty::Bool, None, "file_write_text"),
    );
    m.insert(
        ("Instant".into(), "nowMillis".into()),
        (vec![], Ty::Int, None, "instant_now_ms"),
    );
    m.insert(
        ("Instant".into(), "now".into()),
        (vec![], Ty::Int, None, "instant_now_ms"),
    );
    m.insert(
        ("Dictionary".into(), "new".into()),
        (vec![], Ty::Named("Dictionary".into()), None, "dict_new"),
    );
    m.insert(
        ("Set".into(), "new".into()),
        (vec![], Ty::Named("Set".into()), None, "set_new"),
    );
    m.insert(
        ("Process".into(), "exit".into()),
        (vec![Ty::Int], Ty::Never, None, "process_exit"),
    );
    m.insert(
        ("Process".into(), "sleepMillis".into()),
        (vec![Ty::Int], Ty::Void, None, "sleep_ms"),
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
        ("Http".into(), "post".into()),
        (
            vec![Ty::String, Ty::String],
            Ty::String,
            Some(Ty::Named("HttpError".into())),
            "http_post",
        ),
    );
    m.insert(
        ("Log".into(), "debug".into()),
        (vec![Ty::String], Ty::Void, None, "log_debug"),
    );
    m.insert(
        ("Log".into(), "info".into()),
        (vec![Ty::String], Ty::Void, None, "log_info"),
    );
    m.insert(
        ("Log".into(), "warn".into()),
        (vec![Ty::String], Ty::Void, None, "log_warn"),
    );
    m.insert(
        ("Log".into(), "error".into()),
        (vec![Ty::String], Ty::Void, None, "log_error"),
    );
    m.insert(
        ("CLI".into(), "usage".into()),
        (vec![Ty::String], Ty::Void, None, "cli_usage"),
    );
    m.insert(
        ("CLI".into(), "hasFlag".into()),
        (vec![Ty::String], Ty::Bool, None, "cli_has_flag"),
    );
    m.insert(
        ("TcpStream".into(), "connect".into()),
        (
            vec![Ty::String, Ty::Int],
            Ty::Named("TcpStream".into()),
            None,
            "tcp_connect",
        ),
    );
    m.insert(
        ("Mutex".into(), "create".into()),
        (vec![], Ty::Named("Mutex".into()), None, "mutex_create"),
    );
    // Phase 10 Async / SQL
    m.insert(
        ("Channel".into(), "create".into()),
        (vec![Ty::Int], Ty::Named("Channel".into()), None, "channel_create"),
    );
    m.insert(
        ("WaitGroup".into(), "create".into()),
        (vec![], Ty::Named("WaitGroup".into()), None, "wait_group_create"),
    );
    m.insert(
        ("TaskGroup".into(), "create".into()),
        (vec![], Ty::Named("TaskGroup".into()), None, "task_group_create"),
    );
    m.insert(
        ("Regex".into(), "isMatch".into()),
        (vec![Ty::String, Ty::String], Ty::Bool, None, "regex_is_match"),
    );
    m.insert(
        ("CancellationToken".into(), "create".into()),
        (
            vec![],
            Ty::Named("CancellationToken".into()),
            None,
            "cancellation_token_create",
        ),
    );
    m.insert(
        ("SQLManager".into(), "open".into()),
        (
            vec![Ty::String],
            Ty::Named("SQLConnection".into()),
            None,
            "sql_open",
        ),
    );
    m.insert(
        ("MainActor".into(), "drain".into()),
        (vec![], Ty::Void, None, "main_actor_drain"),
    );
    // `MainActor.run` is modeled like `@run` via task spawn onto the main queue in a fuller
    // build; MVP exposes drain + the `$MAIN_ACTOR` compile-time constant.
    m.insert(
        ("Crypto".into(), "sha256".into()),
        (vec![Ty::String], Ty::String, None, "crypto_sha256"),
    );
    m.insert(
        ("Crypto".into(), "hmacSha256".into()),
        (vec![Ty::String, Ty::String], Ty::String, None, "crypto_hmac_sha256"),
    );
    m.insert(
        ("HMAC".into(), "sha256".into()),
        (vec![Ty::String, Ty::String], Ty::String, None, "crypto_hmac_sha256"),
    );
    m.insert(
        ("Hash".into(), "sha256".into()),
        (vec![Ty::String], Ty::String, None, "crypto_sha256"),
    );
    m.insert(
        ("Log".into(), "debugFields".into()),
        (vec![Ty::String, Ty::String], Ty::Void, None, "log_debug_fields"),
    );
    m.insert(
        ("HttpServer".into(), "listen".into()),
        (vec![Ty::Int, Ty::String], Ty::Void, None, "http_server_listen"),
    );
    m.insert(
        ("Identity".into(), "parse".into()),
        (vec![Ty::String], Ty::optional(Ty::String), None, "identity_parse"),
    );
    m.insert(
        ("Identity".into(), "shortId".into()),
        (vec![], Ty::String, None, "identity_short_id"),
    );
    m.insert(
        ("Arena".into(), "create".into()),
        (vec![Ty::Int], Ty::Named("Arena".into()), None, "arena_create"),
    );
    m.insert(
        ("runBlocking".into(), "call".into()),
        (vec![], Ty::Void, None, "run_blocking_noop"),
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
        ("String".into(), "contains".into()),
        (Ty::Bool, false, "string_contains"),
    );
    m.insert(
        ("String".into(), "trim".into()),
        (Ty::String, false, "string_trim"),
    );
    m.insert(
        ("String".into(), "substring".into()),
        (Ty::String, false, "string_substring"),
    );
    m.insert(
        ("String".into(), "split".into()),
        (Ty::Array(Box::new(Ty::String)), false, "string_split"),
    );
    m.insert(
        ("String".into(), "uppercased".into()),
        (Ty::String, false, "string_upper"),
    );
    m.insert(
        ("String".into(), "lowercased".into()),
        (Ty::String, false, "string_lower"),
    );
    m.insert(
        ("String".into(), "replacing".into()),
        (Ty::String, false, "string_replace"),
    );
    m.insert(
        ("Array".into(), "joined".into()),
        (Ty::String, false, "string_join"),
    );
    m.insert(
        ("Dictionary".into(), "set".into()),
        (Ty::Void, false, "dict_set"),
    );
    m.insert(
        ("Dictionary".into(), "get".into()),
        (Ty::optional(Ty::String), false, "dict_get"),
    );
    m.insert(
        ("Dictionary".into(), "contains".into()),
        (Ty::Bool, false, "dict_contains"),
    );
    m.insert(
        ("Dictionary".into(), "count".into()),
        (Ty::Int, true, "dict_count"),
    );
    m.insert(
        ("Dictionary".into(), "keys".into()),
        (Ty::Array(Box::new(Ty::String)), false, "dict_keys"),
    );
    m.insert(
        ("Set".into(), "insert".into()),
        (Ty::Void, false, "set_insert"),
    );
    m.insert(
        ("Set".into(), "contains".into()),
        (Ty::Bool, false, "set_contains"),
    );
    m.insert(
        ("Set".into(), "count".into()),
        (Ty::Int, true, "set_count"),
    );
    m.insert(
        ("Double".into(), "squareRoot".into()),
        (Ty::Double, false, "double_sqrt"),
    );
    m.insert(
        ("TcpStream".into(), "read".into()),
        (Ty::String, false, "tcp_read"),
    );
    m.insert(
        ("TcpStream".into(), "write".into()),
        (Ty::Void, false, "tcp_write"),
    );
    m.insert(
        ("TcpStream".into(), "close".into()),
        (Ty::Void, false, "tcp_close"),
    );
    m.insert(
        ("Mutex".into(), "lock".into()),
        (Ty::Void, false, "mutex_lock"),
    );
    m.insert(
        ("Mutex".into(), "unlock".into()),
        (Ty::Void, false, "mutex_unlock"),
    );
    m.insert(
        ("Channel".into(), "send".into()),
        (Ty::Void, false, "channel_send_i64"),
    );
    m.insert(
        ("Channel".into(), "receive".into()),
        (Ty::Int, false, "channel_recv_i64"),
    );
    m.insert(
        ("Channel".into(), "close".into()),
        (Ty::Void, false, "channel_close"),
    );
    m.insert(
        ("WaitGroup".into(), "add".into()),
        (Ty::Void, false, "wait_group_add"),
    );
    m.insert(
        ("WaitGroup".into(), "done".into()),
        (Ty::Void, false, "wait_group_done"),
    );
    m.insert(
        ("WaitGroup".into(), "wait".into()),
        (Ty::Void, false, "wait_group_wait"),
    );
    m.insert(
        ("TaskGroup".into(), "wait".into()),
        (Ty::Void, false, "task_group_wait"),
    );
    m.insert(
        ("Task".into(), "join".into()),
        (Ty::Void, false, "task_join"),
    );
    m.insert(
        ("Task".into(), "cancel".into()),
        (Ty::Void, false, "task_cancel"),
    );
    m.insert(
        ("CancellationToken".into(), "cancel".into()),
        (Ty::Void, false, "cancellation_token_cancel"),
    );
    m.insert(
        ("CancellationToken".into(), "throwIfCancelled".into()),
        (Ty::Void, false, "cancellation_token_throw_if_cancelled"),
    );
    m.insert(
        ("SQLConnection".into(), "execute".into()),
        (Ty::String, false, "sql_exec"),
    );
    m.insert(
        ("SQLConnection".into(), "close".into()),
        (Ty::Void, false, "sql_close"),
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
        "Sendable",
    ]
}

/// Error types injected for stdlib MVP.
pub fn prelude_error_types() -> Vec<&'static str> {
    vec!["DecodingError", "EncodingError", "HttpError", "LlmError"]
}
