//! `lang test` — discover suites, desugar assertion macros, cache harness, run in parallel.

use anyhow::{bail, Context, Result};
use lllang_syntax::{parse, Decl, Expr, FuncDecl, MemberDecl, Span, StructDecl};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use crate::report;

/// Assertion macro names (statement-prefix `@name(...)`).
const ASSERT_MACROS: &[&str] = &[
    "expect",
    "true",
    "false",
    "equal",
    "notequal",
    "empty",
    "notEmpty",
    "noempty",
    "gt",
    "gte",
    "lt",
    "lte",
    "contains",
    "approx",
];

#[derive(Debug, Clone)]
pub struct TestCase {
    pub id: usize,
    pub suite: String,
    pub name: String,
    /// When false, cases in this suite share one sequential worker.
    pub parallel_ok: bool,
    /// When true, the test must throw; failing to throw fails the test.
    pub expect_throws: bool,
    /// Optional suite `@fixture` method invoked before the test body.
    pub fixture: Option<String>,
    pub call: TestCall,
}

#[derive(Debug, Clone)]
pub enum TestCall {
    /// Free function name.
    Func(String),
    /// `Type().method(args...)`
    Method {
        type_name: String,
        method: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct TestPlan {
    pub cases: Vec<TestCase>,
    /// Desugared + theory-ready sources to compile with the harness.
    pub units: Vec<(String, String)>,
    pub cache_key: String,
}

pub fn desugar_assertions(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 64);
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            if let Some((name, args_start, args_end)) = match_macro(bytes, i) {
                if ASSERT_MACROS.contains(&name) {
                    let args = &source[args_start..args_end];
                    out.push_str(&rewrite_macro(name, args));
                    i = args_end + 1; // skip closing ')'
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn match_macro(bytes: &[u8], at: usize) -> Option<(&str, usize, usize)> {
    // @name(
    let mut j = at + 1;
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
        j += 1;
    }
    if j >= bytes.len() || bytes[j] != b'(' {
        return None;
    }
    let name = std::str::from_utf8(&bytes[at + 1..j]).ok()?;
    let args_start = j + 1;
    let mut depth = 1i32;
    let mut k = args_start;
    while k < bytes.len() {
        match bytes[k] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((name, args_start, k));
                }
            }
            b'"' => {
                k += 1;
                while k < bytes.len() && bytes[k] != b'"' {
                    if bytes[k] == b'\\' {
                        k += 1;
                    }
                    k += 1;
                }
            }
            _ => {}
        }
        k += 1;
    }
    None
}

fn rewrite_macro(name: &str, args: &str) -> String {
    let args = args.trim();
    let msg = format!("{name}");
    match name {
        "expect" | "true" => format!("__test_expect(({args}), \"{msg}\")"),
        "false" => format!("__test_expect(!({args}), \"{msg}\")"),
        "equal" => {
            let (a, b) = split_two(args);
            format!("__test_expect(({a}) == ({b}), \"{msg}\")")
        }
        "notequal" => {
            let (a, b) = split_two(args);
            format!("__test_expect(!(({a}) == ({b})), \"{msg}\")")
        }
        "empty" => format!("__test_expect_empty(({args}), \"{msg}\")"),
        "notEmpty" | "noempty" => format!("__test_expect_not_empty(({args}), \"{msg}\")"),
        "gt" => {
            let (a, b) = split_two(args);
            format!("__test_expect(({a}) > ({b}), \"{msg}\")")
        }
        "gte" => {
            let (a, b) = split_two(args);
            format!("__test_expect(({a}) >= ({b}), \"{msg}\")")
        }
        "lt" => {
            let (a, b) = split_two(args);
            format!("__test_expect(({a}) < ({b}), \"{msg}\")")
        }
        "lte" => {
            let (a, b) = split_two(args);
            format!("__test_expect(({a}) <= ({b}), \"{msg}\")")
        }
        "contains" => {
            let (a, b) = split_two(args);
            format!("__test_contains(({a}), ({b}), \"{msg}\")")
        }
        "approx" => {
            let (a, b) = split_two(args);
            format!("__test_approx(({a}), ({b}), \"{msg}\")")
        }
        _ => format!("__test_expect(false, \"unknown macro {name}\")"),
    }
}

fn split_two(args: &str) -> (String, String) {
    let mut depth = 0i32;
    for (i, ch) in args.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                return (args[..i].trim().into(), args[i + 1..].trim().into());
            }
            _ => {}
        }
    }
    (args.trim().into(), "0".into())
}

pub fn build_plan(
    package_name: &str,
    lib_sources: &[(String, String)],
    test_sources: &[(String, String)],
) -> Result<TestPlan> {
    let mut units: Vec<(String, String)> = lib_sources.to_vec();
    let mut cases: Vec<TestCase> = Vec::new();
    let mut next_id = 0usize;

    for (path, raw) in test_sources {
        let desugared = desugar_assertions(raw);
        let parsed = parse(&desugared);
        collect_cases(
            &desugared,
            &parsed.file.decls,
            &mut cases,
            &mut next_id,
        );
        units.push((path.clone(), desugared));
    }

    let cache_key = content_hash(package_name, &units);
    Ok(TestPlan {
        cases,
        units,
        cache_key,
    })
}

fn collect_cases(source: &str, decls: &[Decl], cases: &mut Vec<TestCase>, next_id: &mut usize) {
    for d in decls {
        match d {
            Decl::Struct(s) => collect_suite(source, s, cases, next_id),
            Decl::Func(f) => {
                if has_attr(&f.attrs, "test") || has_attr(&f.attrs, "Test") {
                    let title = attr_title(&f.attrs, &["test", "Test"]).unwrap_or_else(|| f.name.clone());
                    cases.push(TestCase {
                        id: *next_id,
                        suite: "(root)".into(),
                        name: title,
                        parallel_ok: true,
                        expect_throws: has_attr(&f.attrs, "throws"),
                        fixture: None,
                        call: TestCall::Func(f.name.clone()),
                    });
                    *next_id += 1;
                }
            }
            _ => {}
        }
    }
}

fn collect_suite(source: &str, s: &StructDecl, cases: &mut Vec<TestCase>, next_id: &mut usize) {
    let Some(suite_attr) = s.attrs.iter().find(|a| a.name == "suite") else {
        return;
    };
    let suite_name = string_lit_arg(&suite_attr.args, 0).unwrap_or_else(|| s.name.clone());
    // `parallel: false` / `parallel = false` → a bool arg is false.
    let parallel_ok = !suite_attr
        .args
        .iter()
        .any(|a| matches!(a, Expr::LiteralBool { value: false, .. }));

    let fixture = s.members.iter().find_map(|m| {
        if let MemberDecl::Func(f) = m {
            if has_attr(&f.attrs, "fixture") {
                return Some(f.name.clone());
            }
        }
        None
    });

    for m in &s.members {
        let MemberDecl::Func(f) = m else { continue };
        if has_attr(&f.attrs, "fixture") {
            continue;
        }
        let expect_throws = has_attr(&f.attrs, "throws");
        if has_attr(&f.attrs, "theory") {
            let title = attr_title(&f.attrs, &["theory"]).unwrap_or_else(|| f.name.clone());
            let rows = values_rows(source, f);
            if rows.is_empty() {
                cases.push(TestCase {
                    id: *next_id,
                    suite: suite_name.clone(),
                    name: title.clone(),
                    parallel_ok,
                    expect_throws,
                    fixture: fixture.clone(),
                    call: TestCall::Method {
                        type_name: s.name.clone(),
                        method: f.name.clone(),
                        args: vec![],
                    },
                });
                *next_id += 1;
            } else {
                for (ri, row) in rows.iter().enumerate() {
                    cases.push(TestCase {
                        id: *next_id,
                        suite: suite_name.clone(),
                        name: format!("{title}[{ri}]"),
                        parallel_ok,
                        expect_throws,
                        fixture: fixture.clone(),
                        call: TestCall::Method {
                            type_name: s.name.clone(),
                            method: f.name.clone(),
                            args: row.clone(),
                        },
                    });
                    *next_id += 1;
                }
            }
        } else if has_attr(&f.attrs, "test") || has_attr(&f.attrs, "Test") {
            let title = attr_title(&f.attrs, &["test", "Test"]).unwrap_or_else(|| f.name.clone());
            cases.push(TestCase {
                id: *next_id,
                suite: suite_name.clone(),
                name: title,
                parallel_ok,
                expect_throws,
                fixture: fixture.clone(),
                call: TestCall::Method {
                    type_name: s.name.clone(),
                    method: f.name.clone(),
                    args: vec![],
                },
            });
            *next_id += 1;
        }
    }
}

fn values_rows(source: &str, f: &FuncDecl) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for a in &f.attrs {
        if a.name != "values" {
            continue;
        }
        let mut row = Vec::new();
        for e in &a.args {
            row.push(span_src(source, expr_span(e)).to_string());
        }
        rows.push(row);
    }
    rows
}

fn expr_span(e: &Expr) -> Span {
    match e {
        Expr::LiteralInt { span, .. }
        | Expr::LiteralFloat { span, .. }
        | Expr::LiteralString { span, .. }
        | Expr::LiteralBool { span, .. }
        | Expr::LiteralNil { span }
        | Expr::LiteralChar { span, .. }
        | Expr::Ident { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Call { span, .. }
        | Expr::Member { span, .. }
        | Expr::Paren { span, .. }
        | Expr::Array { span, .. } => *span,
        _ => Span::EMPTY,
    }
}

fn span_src(source: &str, span: Span) -> &str {
    let start = span.start as usize;
    let end = (span.end as usize).min(source.len()).max(start);
    if start >= source.len() || span.is_empty() {
        // Fallback for empty spans: try Display-ish via Debug of common lits
        "0"
    } else {
        &source[start..end]
    }
}

fn has_attr(attrs: &[lllang_syntax::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.name == name)
}

fn attr_title(attrs: &[lllang_syntax::Attribute], names: &[&str]) -> Option<String> {
    for a in attrs {
        if names.contains(&a.name.as_str()) {
            return string_lit_arg(&a.args, 0);
        }
    }
    None
}

fn string_lit_arg(args: &[Expr], idx: usize) -> Option<String> {
    match args.get(idx) {
        Some(Expr::LiteralString { parts, .. }) => {
            let mut s = String::new();
            for p in parts {
                if let lllang_syntax::StringPart::Text(t) = p {
                    s.push_str(t);
                }
            }
            Some(s)
        }
        _ => None,
    }
}

fn content_hash(package: &str, units: &[(String, String)]) -> String {
    let mut h = Sha256::new();
    h.update(env!("CARGO_PKG_VERSION").as_bytes());
    h.update(package.as_bytes());
    let mut sorted: BTreeMap<&str, &str> = BTreeMap::new();
    for (p, s) in units {
        sorted.insert(p.as_str(), s.as_str());
    }
    for (p, s) in sorted {
        h.update(p.as_bytes());
        h.update(s.as_bytes());
    }
    hex::encode(h.finalize())[..16].to_string()
}

pub fn emit_harness(plan: &TestPlan) -> String {
    let mut out = String::new();
    out.push_str("// Generated by lang test — do not edit\n");
    out.push_str("namespace LangTestHarness\n\n");
    out.push_str("import Std.IO\n");
    out.push_str("import Std.Testing\n\n");

    // Re-emit user units except we already compile them as separate units in analyze.
    // Harness only contains dispatch main + job runners.

    for case in &plan.cases {
        out.push_str(&format!("public func __lang_test_job_{}() {{\n", case.id));
        let mut body = String::new();
        match &case.call {
            TestCall::Func(name) => {
                body.push_str(&format!("    {name}()\n"));
            }
            TestCall::Method {
                type_name,
                method,
                args,
            } => {
                body.push_str(&format!("    let __s = {type_name}()\n"));
                if let Some(fixture_name) = &case.fixture {
                    body.push_str(&format!("    __s.{fixture_name}()\n"));
                }
                // `try!` traps on throw (plain `try` does not rethrow in MVP ABI).
                let try_prefix = if case.expect_throws { "try! " } else { "" };
                if args.is_empty() {
                    body.push_str(&format!("    {try_prefix}__s.{method}()\n"));
                } else {
                    body.push_str(&format!(
                        "    {try_prefix}__s.{method}({})\n",
                        args.join(", ")
                    ));
                }
            }
        }
        // `@throws`: `try` propagates to `main` which traps → non-zero exit; `run_one` inverts.
        out.push_str(&body);
        out.push_str("}\n\n");
    }

    out.push_str("public func main() {\n");
    out.push_str("    let job = __test_job_id()\n");
    for case in &plan.cases {
        out.push_str(&format!(
            "    if job == {} {{ __lang_test_job_{}() }}\n",
            case.id, case.id
        ));
    }
    out.push_str("}\n");
    out
}

pub fn cache_dir(pkg_root: &Path, key: &str) -> PathBuf {
    pkg_root.join(".lang/cache/test").join(key)
}

pub struct RunOpts {
    pub filter: Option<String>,
    pub jobs: Option<usize>,
    pub sequential: bool,
    pub bin: PathBuf,
    pub cache_hit: bool,
}

pub fn run_cases(plan: &TestPlan, opts: &RunOpts) -> Result<usize> {
    let cases: Vec<&TestCase> = plan
        .cases
        .iter()
        .filter(|c| {
            opts.filter.as_ref().map_or(true, |f| {
                c.name.contains(f) || c.suite.contains(f) || format!("{}", c.id).contains(f)
            })
        })
        .collect();
    if cases.is_empty() {
        report::status("Finished", "no tests matched");
        return Ok(0);
    }

    if opts.cache_hit {
        report::status("Finished", format!("cache hit {}", opts.bin.display()));
    }

    report::status(
        "Running",
        format!(
            "{} test{}",
            cases.len(),
            if cases.len() == 1 { "" } else { "s" }
        ),
    );

    let jobs = if opts.sequential {
        1
    } else {
        opts.jobs
            .unwrap_or_else(|| thread::available_parallelism().map(|n| n.get()).unwrap_or(4))
    };

    // Partition: sequential suites run on a dedicated lane; parallel cases use the pool.
    let (seq, par): (Vec<_>, Vec<_>) = cases
        .into_iter()
        .partition(|c| !c.parallel_ok || opts.sequential);

    let failed = Arc::new(AtomicUsize::new(0));
    let passed = Arc::new(AtomicUsize::new(0));

    // Sequential suites first (ordered).
    for c in &seq {
        if !run_one(&opts.bin, c)? {
            failed.fetch_add(1, Ordering::Relaxed);
            report::status_err("error", format!("{}::{}", c.suite, c.name));
        } else {
            passed.fetch_add(1, Ordering::Relaxed);
            report::check_ok_test(&format!("{}::{}", c.suite, c.name));
        }
    }

    // Parallel pool.
    let bin = opts.bin.clone();
    let chunk = std::cmp::max(1, par.len() / jobs.max(1));
    let _ = chunk;
    thread::scope(|scope| {
        let workers = jobs.min(par.len().max(1));
        let queue = Arc::new(std::sync::Mutex::new(par.into_iter().collect::<Vec<_>>()));
        for _ in 0..workers {
            let queue = Arc::clone(&queue);
            let bin = bin.clone();
            let failed = Arc::clone(&failed);
            let passed = Arc::clone(&passed);
            scope.spawn(move || {
                loop {
                    let next = {
                        let mut q = queue.lock().unwrap();
                        q.pop()
                    };
                    let Some(c) = next else { break };
                    match run_one(&bin, &c) {
                        Ok(true) => {
                            passed.fetch_add(1, Ordering::Relaxed);
                            report::check_ok_test(&format!("{}::{}", c.suite, c.name));
                        }
                        Ok(false) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            report::status_err("error", format!("{}::{}", c.suite, c.name));
                        }
                        Err(e) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                            report::status_err("error", format!("{}::{} ({e:#})", c.suite, c.name));
                        }
                    }
                }
            });
        }
    });

    let f = failed.load(Ordering::Relaxed);
    let p = passed.load(Ordering::Relaxed);
    if f > 0 {
        report::status_err(
            "error",
            format!("test result: {p} passed; {f} failed"),
        );
        bail!("{f} test(s) failed");
    }
    report::status("Finished", format!("test result: {p} passed; 0 failed"));
    Ok(p)
}

fn run_one(bin: &Path, case: &TestCase) -> Result<bool> {
    let status = Command::new(bin)
        .env("LANG_TEST_JOB", case.id.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .with_context(|| format!("spawn {}", bin.display()))?;
    if case.expect_throws {
        Ok(!status.success())
    } else {
        Ok(status.success())
    }
}

/// Write harness + return path to harness.ll
pub fn write_cache_inputs(dir: &Path, plan: &TestPlan) -> Result<PathBuf> {
    fs::create_dir_all(dir)?;
    for (i, (path, src)) in plan.units.iter().enumerate() {
        let name = Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unit.ll");
        let dest = dir.join(format!("{i}_{name}"));
        fs::write(&dest, src)?;
    }
    let harness = emit_harness(plan);
    let harness_path = dir.join("harness.ll");
    fs::write(&harness_path, harness)?;
    let manifest = format!("cases: {}\nkey: {}\n", plan.cases.len(), plan.cache_key);
    fs::write(dir.join("manifest.toon"), manifest)?;
    Ok(harness_path)
}
