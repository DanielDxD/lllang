//! Parse `Package.ll` and `Solution.ll` manifests from LLLang source.

use anyhow::Context;
use lllang_syntax::{parse, CallArg, Decl, Expr, File, Pattern, Severity};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MANIFEST_ERROR_CODE: &str = "LL5000";

/// Stable manifest diagnostic code (see `docs/spec/diagnostics.md`).
pub const MANIFEST_DIAGNOSTIC_CODE: &str = MANIFEST_ERROR_CODE;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifest {
    pub name: String,
    pub products: Vec<Product>,
    pub targets: Vec<Target>,
    pub dependencies: Vec<PackageDep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Product {
    Executable {
        name: String,
        targets: Vec<String>,
    },
    Library {
        name: String,
        targets: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Executable,
    Library,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub name: String,
    pub kind: TargetKind,
    pub path: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageDep {
    Path { path: String },
    Registry { id: String, from: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolutionManifest {
    pub name: String,
    pub projects: Vec<SolutionProject>,
    pub default_projects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolutionProject {
    pub path: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}")]
pub struct ManifestError {
    pub code: String,
    pub message: String,
}

impl ManifestError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            code: MANIFEST_ERROR_CODE.to_string(),
            message: message.into(),
        }
    }
}

/// Load `Package.ll` from disk.
pub fn load_package(path: &Path) -> Result<PackageManifest, ManifestError> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading manifest {}", path.display()))
        .map_err(|e| ManifestError::new(e.to_string()))?;
    let hint = path.to_string_lossy();
    load_package_source(&source, &hint)
}

/// Parse `Package.ll` source text.
pub fn load_package_source(source: &str, path_hint: &str) -> Result<PackageManifest, ManifestError> {
    let file = parse_checked(source, path_hint)?;
    let init = find_let_initializer(&file, "package")?;
    extract_package(&init)
}

/// Load `Solution.ll` from disk.
pub fn load_solution(path: &Path) -> Result<SolutionManifest, ManifestError> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("reading solution {}", path.display()))
        .map_err(|e| ManifestError::new(e.to_string()))?;
    let hint = path.to_string_lossy();
    load_solution_source(&source, &hint)
}

fn load_solution_source(source: &str, path_hint: &str) -> Result<SolutionManifest, ManifestError> {
    let file = parse_checked(source, path_hint)?;
    let init = find_let_initializer(&file, "solution")?;
    extract_solution(&init)
}

/// Collect `.ll` source files for each target under its configured or default directory.
pub fn discover_sources(
    package_dir: &Path,
    manifest: &PackageManifest,
) -> Result<Vec<PathBuf>, ManifestError> {
    let mut paths = Vec::new();
    for target in &manifest.targets {
        let dir = target_source_dir(package_dir, target);
        if dir.is_dir() {
            collect_ll_files(&dir, &mut paths).map_err(|e| {
                ManifestError::new(format!(
                    "failed to scan sources under {}: {e}",
                    dir.display()
                ))
            })?;
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

/// Walk upward from `start` until a directory containing `Package.ll` is found.
pub fn find_package_root(start: &Path) -> Option<PathBuf> {
    find_manifest_root(start, "Package.ll")
}

/// Walk upward from `start` until a directory containing `Solution.ll` is found.
pub fn find_solution_root(start: &Path) -> Option<PathBuf> {
    find_manifest_root(start, "Solution.ll")
}

fn find_manifest_root(start: &Path, file_name: &str) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if dir.join(file_name).is_file() {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
    }
}

fn target_source_dir(package_dir: &Path, target: &Target) -> PathBuf {
    if let Some(path) = &target.path {
        return package_dir.join(path);
    }
    let sources = package_dir.join("Sources").join(&target.name);
    if sources.is_dir() {
        sources
    } else {
        package_dir.join("src")
    }
}

fn collect_ll_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_ll_files(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ll"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn parse_checked(source: &str, path_hint: &str) -> Result<File, ManifestError> {
    let result = parse(source);
    let errors: Vec<_> = result
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    if !errors.is_empty() {
        let first = &errors[0];
        return Err(ManifestError::new(format!(
            "syntax error in {path_hint}: {} ({})",
            first.message, first.code
        )));
    }
    Ok(result.file)
}

fn find_let_initializer(file: &File, binding_name: &str) -> Result<Expr, ManifestError> {
    for decl in &file.decls {
        let Decl::Let(let_decl) = decl else {
            continue;
        };
        for binding in &let_decl.bindings {
            if pattern_name(&binding.pattern).as_deref() == Some(binding_name) {
                if let Some(init) = &binding.initializer {
                    return Ok(init.clone());
                }
                return Err(ManifestError::new(format!(
                    "binding `{binding_name}` has no initializer"
                )));
            }
        }
    }
    Err(ManifestError::new(format!(
        "expected `let {binding_name} = ...` at top level"
    )))
}

fn pattern_name(pattern: &Pattern) -> Option<String> {
    match pattern {
        Pattern::Ident { name, .. } => Some(name.clone()),
        Pattern::Binding { pattern, .. } => pattern_name(pattern),
        Pattern::Typed { pattern, .. } => pattern_name(pattern),
        _ => None,
    }
}

fn extract_package(expr: &Expr) -> Result<PackageManifest, ManifestError> {
    let args = top_level_call_args(expr, "Package")?;
    let name = required_string_arg(&args, "name")?;
    let products = args
        .get("products")
        .map(|e| extract_products(e))
        .transpose()?
        .unwrap_or_default();
    let targets = args
        .get("targets")
        .map(|e| extract_targets(e))
        .transpose()?
        .unwrap_or_default();
    let dependencies = args
        .get("dependencies")
        .map(|e| extract_package_deps(e))
        .transpose()?
        .unwrap_or_default();
    Ok(PackageManifest {
        name,
        products,
        targets,
        dependencies,
    })
}

fn extract_solution(expr: &Expr) -> Result<SolutionManifest, ManifestError> {
    let args = top_level_call_args(expr, "Solution")?;
    let name = required_string_arg(&args, "name")?;
    let projects = args
        .get("projects")
        .map(|e| extract_solution_projects(e))
        .transpose()?
        .unwrap_or_default();
    let default_projects = args
        .get("defaultProjects")
        .and_then(expr_as_string_list)
        .unwrap_or_default();
    Ok(SolutionManifest {
        name,
        projects,
        default_projects,
    })
}

fn top_level_call_args(expr: &Expr, expected: &str) -> Result<HashMap<String, Expr>, ManifestError> {
    let Expr::Call { callee, args, .. } = expr else {
        return Err(ManifestError::new(format!(
            "expected `{expected}(...)` call expression"
        )));
    };
    let Some(callee_name) = expr_ident_name(callee) else {
        return Err(ManifestError::new(format!(
            "expected `{expected}(...)` call expression"
        )));
    };
    if callee_name != expected {
        return Err(ManifestError::new(format!(
            "expected `{expected}(...)`, found `{callee_name}(...)`"
        )));
    }
    Ok(labeled_args(args))
}

fn labeled_args(args: &[CallArg]) -> HashMap<String, Expr> {
    let mut map = HashMap::new();
    for arg in args {
        if let Some(label) = &arg.label {
            map.insert(label.clone(), arg.value.clone());
        }
    }
    map
}

fn required_string_arg(args: &HashMap<String, Expr>, name: &str) -> Result<String, ManifestError> {
    args.get(name)
        .and_then(expr_as_string)
        .ok_or_else(|| ManifestError::new(format!("missing or invalid `{name}:` argument")))
}

fn extract_products(expr: &Expr) -> Result<Vec<Product>, ManifestError> {
    let elements = expr_as_array(expr).ok_or_else(|| {
        ManifestError::new("products must be an array of product descriptions")
    })?;
    elements.iter().map(parse_product).collect()
}

fn parse_product(expr: &Expr) -> Result<Product, ManifestError> {
    let (factory, args) = factory_call(expr)?;
    match factory.as_str() {
        "executable" | ".executable" => Ok(Product::Executable {
            name: required_string_arg(&args, "name")?,
            targets: args
                .get("targets")
                .and_then(expr_as_string_list)
                .unwrap_or_default(),
        }),
        "library" | ".library" => Ok(Product::Library {
            name: required_string_arg(&args, "name")?,
            targets: args
                .get("targets")
                .and_then(expr_as_string_list)
                .unwrap_or_default(),
        }),
        other => Err(ManifestError::new(format!(
            "unknown product factory `{other}`"
        ))),
    }
}

fn extract_targets(expr: &Expr) -> Result<Vec<Target>, ManifestError> {
    let elements = expr_as_array(expr)
        .ok_or_else(|| ManifestError::new("targets must be an array of target descriptions"))?;
    elements.iter().map(parse_target).collect()
}

fn parse_target(expr: &Expr) -> Result<Target, ManifestError> {
    let (factory, args) = factory_call(expr)?;
    let kind = match factory.as_str() {
        "executableTarget" | ".executableTarget" => TargetKind::Executable,
        "target" | ".target" => TargetKind::Library,
        "testTarget" | ".testTarget" => TargetKind::Test,
        "binaryTarget" | ".binaryTarget" => TargetKind::Library,
        other => {
            return Err(ManifestError::new(format!(
                "unknown target factory `{other}`"
            )))
        }
    };
    let name = required_string_arg(&args, "name")?;
    let path = args.get("path").and_then(expr_as_string);
    let dependencies = args
        .get("dependencies")
        .map(|e| extract_target_deps(e))
        .transpose()?
        .unwrap_or_default();
    Ok(Target {
        name,
        kind,
        path,
        dependencies,
    })
}

fn extract_package_deps(expr: &Expr) -> Result<Vec<PackageDep>, ManifestError> {
    let elements = expr_as_array(expr).ok_or_else(|| {
        ManifestError::new("dependencies must be an array of package dependencies")
    })?;
    elements.iter().map(parse_package_dep).collect()
}

fn parse_package_dep(expr: &Expr) -> Result<PackageDep, ManifestError> {
    let (factory, args) = factory_call(expr)?;
    if factory == "package" || factory == ".package" {
        if let Some(path) = args.get("path").and_then(expr_as_string) {
            return Ok(PackageDep::Path { path });
        }
        if let Some(url) = args.get("url").and_then(expr_as_string) {
            return Ok(PackageDep::Path { path: url });
        }
        let id = args
            .get("id")
            .and_then(expr_as_string)
            .ok_or_else(|| ManifestError::new("registry package dependency requires `id:`"))?;
        let from = args
            .get("from")
            .and_then(expr_as_string)
            .or_else(|| args.get("exact").and_then(expr_as_string))
            .or_else(|| args.get("range").and_then(expr_as_string));
        return Ok(PackageDep::Registry { id, from });
    }
    Err(ManifestError::new(format!(
        "unknown package dependency factory `{factory}`"
    )))
}

fn extract_solution_projects(expr: &Expr) -> Result<Vec<SolutionProject>, ManifestError> {
    let elements = expr_as_array(expr).ok_or_else(|| {
        ManifestError::new("projects must be an array of solution project entries")
    })?;
    elements.iter().map(parse_solution_project).collect()
}

fn parse_solution_project(expr: &Expr) -> Result<SolutionProject, ManifestError> {
    let (factory, args) = factory_call(expr)?;
    if factory != "package" && factory != ".package" {
        return Err(ManifestError::new(format!(
            "unknown solution project factory `{factory}`"
        )));
    }
    let path = required_string_arg(&args, "path")?;
    let name = args.get("name").and_then(expr_as_string);
    Ok(SolutionProject { path, name })
}

fn extract_target_deps(expr: &Expr) -> Result<Vec<String>, ManifestError> {
    let Some(elements) = expr_as_array(expr) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for el in elements {
        if let Some(s) = expr_as_string(&el) {
            out.push(s);
            continue;
        }
        if let Ok((factory, args)) = factory_call(&el) {
            match factory.as_str() {
                "byName" | ".byName" => {
                    if let Some(name) = args.values().find_map(expr_as_string) {
                        out.push(name);
                    }
                }
                "target" | ".target" => {
                    if let Some(name) = args.get("name").and_then(expr_as_string) {
                        out.push(name);
                    }
                }
                "product" | ".product" => {
                    if let Some(name) = args.get("name").and_then(expr_as_string) {
                        out.push(name);
                    } else if let Some(pkg) = args.get("package").and_then(expr_as_string) {
                        out.push(pkg);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(out)
}

fn factory_call(expr: &Expr) -> Result<(String, HashMap<String, Expr>), ManifestError> {
    match expr {
        Expr::Call { callee, args, .. } => {
            let name = expr_ident_name(callee).ok_or_else(|| {
                ManifestError::new("expected factory call such as `.executable(...)`")
            })?;
            Ok((name, labeled_args(args)))
        }
        _ => Err(ManifestError::new(
            "expected factory call such as `.executable(...)`",
        )),
    }
}

fn expr_ident_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn expr_as_array(expr: &Expr) -> Option<Vec<Expr>> {
    match expr {
        Expr::Array { elements, .. } => Some(elements.clone()),
        _ => None,
    }
}

fn expr_as_string_list(expr: &Expr) -> Option<Vec<String>> {
    expr_as_array(expr)?
        .iter()
        .map(expr_as_string)
        .collect()
}

fn expr_as_string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::LiteralString { parts, .. } => {
            let mut s = String::new();
            for part in parts {
                match part {
                    lllang_syntax::StringPart::Text(t) => s.push_str(t),
                    lllang_syntax::StringPart::Interpolation(_) => return None,
                }
            }
            Some(s)
        }
        _ => None,
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn parse_minimal_package() {
        let src = r#"
let package = Package(
    name: "demo",
    products: [.executable(name: "demo", targets: ["Demo"])],
    targets: [.executableTarget(name: "Demo", path: "src")],
)
"#;
        let m = load_package_source(src, "Package.ll").unwrap();
        assert_eq!(m.name, "demo");
        assert_eq!(m.products.len(), 1);
        assert_eq!(m.targets.len(), 1);
    }

    #[test]
    fn shop_cli_dependencies() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/shop-solution/apps/Cli/Package.ll");
        let m = load_package(&path).unwrap();
        assert_eq!(m.name, "shop.cli");
        assert!(!m.dependencies.is_empty());
        assert!(!m.targets[0].dependencies.is_empty());
    }
}
