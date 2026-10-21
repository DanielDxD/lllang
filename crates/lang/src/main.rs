//! LLLang CLI (`lang`) — Phase 2: check (sema), build, run, lsp.

mod report;
mod test_runner;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use lllang_manifest::{
    discover_sources, find_package_root, find_solution_root, load_package, load_solution, TargetKind,
};
use report::{ColorChoice, Timer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "lang", version = VERSION, about = "LLLang toolchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long)]
    info: bool,

    /// When to use colored output: auto, always, never
    #[arg(long, global = true, default_value = "auto", value_name = "WHEN")]
    color: String,

    #[arg(long, global = true)]
    manifest: Option<PathBuf>,

    #[arg(long, global = true)]
    solution: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    New {
        template: Option<String>,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        #[arg(long)]
        list: bool,
    },
    Build {
        #[arg(long)]
        release: bool,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        emit_llvm: bool,
        #[arg(long)]
        emit_mir: bool,
    },
    Run {
        #[arg(long)]
        release: bool,
        #[arg(long)]
        project: Option<String>,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Watch,
    Test {
        filter: Option<String>,
        #[arg(long)]
        jobs: Option<usize>,
        #[arg(long)]
        sequential: bool,
    },
    Bench,
    Check {
        paths: Vec<PathBuf>,
    },
    Fmt {
        paths: Vec<PathBuf>,
        #[arg(long)]
        check: bool,
        #[arg(long, default_value_t = true)]
        write: bool,
        #[arg(long)]
        workspace: bool,
    },
    Doc,
    Clean,
    Add { package: String },
    Remove { package: String },
    Restore,
    Update,
    Pack {
        #[arg(long)]
        targets: Option<String>,
    },
    Publish {
        #[arg(long)]
        registry: Option<String>,
    },
    Search {
        query: String,
        #[arg(long)]
        registry: Option<String>,
    },
    Llm {
        #[command(subcommand)]
        cmd: LlmCmd,
    },
    Sdk {
        #[command(subcommand)]
        cmd: SdkCmd,
    },
    Lsp,
    Solution {
        #[command(subcommand)]
        cmd: SolutionCmd,
    },
}

#[derive(Subcommand, Debug)]
enum SolutionCmd {
    List,
    Add { path: PathBuf },
    Remove { name: String },
}

#[derive(Subcommand, Debug)]
enum LlmCmd {
    Gen,
    Context {
        #[arg(long)]
        budget: Option<u32>,
    },
    Explain { symbol: String },
}

#[derive(Subcommand, Debug)]
enum SdkCmd {
    List,
    Install { version: String },
    Use { version: String },
}

fn planned(feature: &str) -> Result<()> {
    report::status_err(
        "note",
        format!("`{feature}` is planned for a later phase — see ROADMAP.md"),
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            report::fatal(&err);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let choice = ColorChoice::parse(&cli.color).unwrap_or(ColorChoice::Auto);
    report::set_color_choice(choice);
    if cli.info {
        print_info();
        return Ok(());
    }
    match cli.command {
        None => {
            Cli::parse_from(["lang", "--help"]);
            Ok(())
        }
        Some(Commands::Lsp) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            rt.block_on(lllang_lsp::run_stdio());
            Ok(())
        }
        Some(Commands::Check { paths }) => cmd_check(paths),
        Some(Commands::Build {
            release,
            target,
            project,
            all,
            emit_llvm,
            emit_mir,
        }) => {
            cmd_build(BuildOpts {
                release,
                target,
                project,
                all,
                emit_llvm,
                emit_mir,
                manifest: cli.manifest.clone(),
                solution: cli.solution.clone(),
            })?;
            Ok(())
        }
        Some(Commands::Run {
            release,
            project,
            args,
        }) => {
            let bin = cmd_build(BuildOpts {
                release,
                target: None,
                project,
                all: false,
                emit_llvm: false,
                emit_mir: false,
                manifest: cli.manifest.clone(),
                solution: cli.solution.clone(),
            })?;
            report::status("Running", report::short_path(&bin));
            let status = Command::new(&bin).args(&args).status()?;
            if !status.success() {
                bail!("program exited with {status}");
            }
            Ok(())
        }
        Some(Commands::New {
            list,
            template,
            name,
            output,
        }) => {
            if list {
                cmd_new_list()
            } else {
                cmd_new(template, name, output)
            }
        }
        Some(Commands::Solution { cmd }) => match cmd {
            SolutionCmd::List => planned("solution list"),
            SolutionCmd::Add { .. } => planned("solution add"),
            SolutionCmd::Remove { .. } => planned("solution remove"),
        },
        Some(Commands::Watch) => planned("watch"),
        Some(Commands::Test {
            filter,
            jobs,
            sequential,
        }) => cmd_test(filter, jobs, sequential, cli.manifest.clone()),
        Some(Commands::Bench) => cmd_bench(),
        Some(Commands::Fmt {
            paths,
            check,
            write,
            workspace,
        }) => cmd_fmt(paths, check, write, workspace),
        Some(Commands::Doc) => planned("doc"),
        Some(Commands::Clean) => {
            let root = std::env::current_dir()?;
            let dir = root.join(".lang");
            if dir.exists() {
                std::fs::remove_dir_all(&dir)?;
                report::status("Removing", report::short_path(&dir));
            } else {
                report::status("Finished", "nothing to clean");
            }
            Ok(())
        }
        Some(Commands::Add { package }) => cmd_add(&package),
        Some(Commands::Remove { package }) => cmd_remove(&package),
        Some(Commands::Restore) => cmd_restore(),
        Some(Commands::Update) => planned("update"),
        Some(Commands::Pack { targets }) => cmd_pack(targets),
        Some(Commands::Publish { registry }) => cmd_publish(registry.as_deref()),
        Some(Commands::Search { query, registry }) => cmd_search(&query, registry.as_deref()),
        Some(Commands::Llm { .. }) => planned("llm"),
        Some(Commands::Sdk { cmd }) => cmd_sdk(cmd),
    }
}

fn print_info() {
    println!("LLLang toolchain {VERSION}");
    println!("phase: 4 (LSP completion, registry, templates)");
    println!("host: {}", std::env::consts::OS);
    println!("arch: {}", std::env::consts::ARCH);
    if let Ok(p) = std::env::var("LLVM_SYS_221_PREFIX") {
        println!("llvm: {p}");
    } else {
        println!("llvm: set LLVM_SYS_221_PREFIX (e.g. /opt/homebrew/opt/llvm@22)");
    }
}

fn cmd_fmt(paths: Vec<PathBuf>, check: bool, write: bool, workspace: bool) -> Result<()> {
    let timer = Timer::start();
    let cwd = std::env::current_dir()?;
    let mut files = Vec::new();
    if paths.is_empty() || workspace {
        let root = if workspace {
            find_solution_root(&cwd)
                .or_else(|| find_package_root(&cwd))
                .unwrap_or(cwd.clone())
        } else {
            find_package_root(&cwd).unwrap_or(cwd.clone())
        };
        collect_ll_sources(&root, &mut files)?;
    } else {
        for p in paths {
            collect_ll_sources(&p, &mut files)?;
        }
    }
    files.retain(|f| {
        let name = f.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name != "Package.ll"
            && name != "Solution.ll"
            && f.extension().and_then(|e| e.to_str()) == Some("ll")
    });
    if files.is_empty() {
        bail!("no .ll files found");
    }
    report::status(
        "Formatting",
        format!("{} file{}", files.len(), if files.len() == 1 { "" } else { "s" }),
    );
    let mut changed = 0usize;
    for file in &files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("reading {}", file.display()))?;
        let formatted = match lllang_syntax::format_source(&source) {
            Ok(s) => s,
            Err(diags) => {
                let mut sources = HashMap::new();
                let key = file.display().to_string();
                sources.insert(key.clone(), source);
                let diags: Vec<_> = diags
                    .into_iter()
                    .map(|d| d.with_file(key.clone()))
                    .collect();
                report::emit_diagnostics(&diags, &sources);
                bail!("format failed for {}", report::short_path(file));
            }
        };
        if formatted == source {
            continue;
        }
        changed += 1;
        if check {
            report::fmt_changed(file, true);
        } else if write {
            std::fs::write(file, &formatted)
                .with_context(|| format!("writing {}", file.display()))?;
            report::fmt_changed(file, false);
        } else {
            print!("{formatted}");
        }
    }
    report::fmt_summary(changed, files.len(), check, timer.elapsed_secs());
    if check && changed > 0 {
        return Err(report::exit_failure());
    }
    Ok(())
}

fn collect_ll_sources(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("ll") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))? {
        let entry = entry?;
        let p = entry.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == ".git" || name == "target" || name == ".lang" || name == "node_modules" {
            continue;
        }
        collect_ll_sources(&p, out)?;
    }
    Ok(())
}

fn cmd_pack(_targets: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).unwrap_or(cwd.clone());
    let manifest = load_package(&root.join("Package.ll"))?;
    let sources = discover_sources(&root, &manifest)?;
    let source_pairs: Vec<(String, String)> = sources
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?;
            if name == "Package.ll" || name == "Solution.ll" {
                return None;
            }
            let src = std::fs::read_to_string(p).ok()?;
            Some((p.display().to_string(), src))
        })
        .collect();
    // Build object first when this is a library with code.
    let build_dir = root.join(".lang").join("pack");
    std::fs::create_dir_all(&build_dir)?;
    let opts = BuildOpts {
        release: true,
        target: None,
        project: None,
        all: false,
        emit_llvm: false,
        emit_mir: false,
        manifest: Some(root.join("Package.ll")),
        solution: None,
    };
    let object_bytes = match build_package(&root, &opts) {
        Ok(bin) => {
            // Prefer .o next to binary if present; else empty stub for surface-only packs.
            let o = bin.with_extension("o");
            if o.exists() {
                std::fs::read(&o).unwrap_or_default()
            } else if bin.exists() {
                std::fs::read(&bin).unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Err(_) => Vec::new(),
    };
    let out = root.join(".lang").join("dist");
    let (sbl_path, bll_path, sbl_hash, bll_hash) = lllang_pack::pack_library(
        &manifest.name,
        "0.1.0",
        &source_pairs,
        &object_bytes,
        &out,
    )?;
    let sbl_bytes = std::fs::read(&sbl_path)?;
    let bll_bytes = std::fs::read(&bll_path)?;
    let entry = lllang_pack::install_to_store(&manifest.name, "0.1.0", &sbl_bytes, &bll_bytes)?;
    // Update Package.lock
    let lock_path = root.join("Package.lock");
    let mut lock = if lock_path.exists() {
        lllang_pack::read_lock(&lock_path).unwrap_or_default()
    } else {
        lllang_pack::LockFile {
            version: 1,
            packages: vec![],
        }
    };
    lock.packages.retain(|p| p.name != manifest.name);
    lock.packages.push(lllang_pack::LockedPackage {
        name: manifest.name.clone(),
        version: "0.1.0".into(),
        sbl: sbl_hash,
        source: format!("path:{}", root.display()),
        targets: vec![lllang_pack::LockedTarget {
            triple: lllang_pack::host_triple(),
            bll: bll_hash,
        }],
    });
    lllang_pack::write_lock(&lock_path, &lock)?;
    report::status("Packing", format!("{} -> {}", manifest.name, report::short_path(&entry)));
    report::status("Finished", format!("sbl {}", report::short_path(&sbl_path)));
    report::status("Finished", format!("bll {}", report::short_path(&bll_path)));
    Ok(())
}

fn cmd_restore() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd)
        .or_else(|| find_solution_root(&cwd))
        .unwrap_or(cwd);
    let lock_path = root.join("Package.lock");
    if !lock_path.exists() {
        report::status("Finished", "no Package.lock; nothing to restore");
        return Ok(());
    }
    let lock = lllang_pack::read_lock(&lock_path)?;
    report::status(
        "Restoring",
        format!("{} package(s)", lock.packages.len()),
    );
    lllang_pack::ensure_restored(&lock)?;
    report::status(
        "Finished",
        format!("restored {} package(s) from store", lock.packages.len()),
    );
    Ok(())
}

fn cmd_add(package: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).unwrap_or(cwd);
    let lock_path = root.join("Package.lock");
    let mut lock = if lock_path.exists() {
        lllang_pack::read_lock(&lock_path).unwrap_or_default()
    } else {
        lllang_pack::LockFile {
            version: 1,
            packages: vec![],
        }
    };
    let (name, version) = if let Some((n, v)) = package.split_once('@') {
        (n.to_string(), v.to_string())
    } else {
        (package.to_string(), "0.1.0".into())
    };
    lock.packages.retain(|p| p.name != name);
    lock.packages.push(lllang_pack::LockedPackage {
        name: name.clone(),
        version: version.clone(),
        sbl: String::new(),
        source: "registry".into(),
        targets: vec![],
    });
    lllang_pack::write_lock(&lock_path, &lock)?;
    report::status(
        "Adding",
        format!("{name}@{version} to Package.lock (fetch on restore when published)"),
    );
    Ok(())
}

fn cmd_remove(package: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).unwrap_or(cwd);
    let lock_path = root.join("Package.lock");
    if !lock_path.exists() {
        bail!("no Package.lock");
    }
    let mut lock = lllang_pack::read_lock(&lock_path)?;
    let before = lock.packages.len();
    lock.packages.retain(|p| p.name != package);
    if lock.packages.len() == before {
        bail!("package `{package}` not in lock");
    }
    lllang_pack::write_lock(&lock_path, &lock)?;
    report::status("Removing", package);
    Ok(())
}

fn cmd_search(query: &str, registry: Option<&str>) -> Result<()> {
    report::status("Searching", query);
    let root = lllang_pack::registry_root(registry);
    let reg = lllang_pack::FileRegistry::open(&root)?;
    let hits = reg.search(query)?;
    if !hits.is_empty() {
        for pkg in &hits {
            println!("  {}@{}", pkg.name, pkg.version);
        }
        report::status(
            "Finished",
            format!(
                "{} match{} in {}",
                hits.len(),
                if hits.len() == 1 { "" } else { "es" },
                root.display()
            ),
        );
        return Ok(());
    }
    let store = lllang_pack::Store::open()?;
    let pkgs = store.root().join("packages");
    let mut n = 0usize;
    if pkgs.exists() {
        for entry in std::fs::read_dir(&pkgs)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.contains(query) {
                println!("  {name} (store)");
                n += 1;
            }
        }
    }
    report::status(
        "Finished",
        format!("{n} store match{}", if n == 1 { "" } else { "es" }),
    );
    Ok(())
}

fn cmd_publish(registry: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).unwrap_or(cwd);
    let manifest = load_package(&root.join("Package.ll"))?;
    let sources = discover_sources(&root, &manifest)?;
    let source_pairs: Vec<(String, String)> = sources
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?;
            if name == "Package.ll" || name == "Solution.ll" {
                return None;
            }
            let src = std::fs::read_to_string(p).ok()?;
            Some((p.display().to_string(), src))
        })
        .collect();
    let out = root.join(".lang").join("dist");
    let object_bytes = Vec::new();
    let (sbl_path, bll_path, _, _) = lllang_pack::pack_library(
        &manifest.name,
        "0.1.0",
        &source_pairs,
        &object_bytes,
        &out,
    )?;
    let sbl_bytes = std::fs::read(&sbl_path)?;
    let bll_bytes = std::fs::read(&bll_path)?;
    let reg_root = lllang_pack::registry_root(registry);
    let reg = lllang_pack::FileRegistry::open(&reg_root)?;
    let pkg = reg.publish(&manifest.name, "0.1.0", &sbl_bytes, &bll_bytes)?;
    lllang_pack::install_to_store(&pkg.name, &pkg.version, &sbl_bytes, &bll_bytes)?;
    report::status(
        "Finished",
        format!(
            "published {}@{} -> {}",
            pkg.name,
            pkg.version,
            reg_root.display()
        ),
    );
    Ok(())
}

const TEMPLATES: &[&str] = &[
    "console", "lib", "cli", "web", "test", "agent", "mcp", "bench", "solution",
];

fn templates_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates")
}

fn cmd_new_list() -> Result<()> {
    println!("Templates: {}", TEMPLATES.join(", "));
    Ok(())
}

fn cmd_new(
    template: Option<String>,
    name: Option<String>,
    output: Option<PathBuf>,
) -> Result<()> {
    let template = template.unwrap_or_else(|| "console".into());
    if !TEMPLATES.contains(&template.as_str()) {
        bail!("unknown template `{template}`; use --list");
    }
    let name = name.unwrap_or_else(|| template.clone());
    let dest = output.unwrap_or_else(|| PathBuf::from(&name));
    if dest.exists() {
        bail!("{} already exists", dest.display());
    }
    let src = templates_dir().join(&template);
    if !src.is_dir() {
        bail!(
            "template `{template}` not found at {} (run from a built lang binary in this repo)",
            src.display()
        );
    }
    copy_template(&src, &dest, &name)?;
    report::status(
        "Finished",
        format!("created {} from template `{template}`", dest.display()),
    );
    Ok(())
}

fn copy_template(src: &Path, dest: &Path, name: &str) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let pascal = to_pascal(name);
    let tests = format!("{pascal}Tests");
    for entry in walkdir_ll(src)? {
        let rel = entry.strip_prefix(src).unwrap();
        let mut s = dest.join(rel).to_string_lossy().to_string();
        // Rename scaffold directories to the generated type name.
        for (from, to) in [
            ("MyBenchTests", tests.as_str()),
            ("LibTests", tests.as_str()),
            ("AppTests", tests.as_str()),
            ("MyAgent", pascal.as_str()),
            ("MyMcp", pascal.as_str()),
            ("MyBench", pascal.as_str()),
            ("Hello", pascal.as_str()),
            ("Sources/Lib", &format!("Sources/{pascal}")),
            ("Sources/Cli", &format!("Sources/{pascal}")),
            ("Sources/Web", &format!("Sources/{pascal}")),
            ("Sources/App", &format!("Sources/{pascal}")),
            ("Tests/Lib", &format!("Tests/{pascal}")),
            ("Tests/App", &format!("Tests/{pascal}")),
        ] {
            s = s.replace(from, to);
        }
        s = s.replace("{{name}}", name).replace("{{Name}}", &pascal);
        let target = PathBuf::from(&s);
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        let text = text
            .replace("{{name}}", name)
            .replace("{{Name}}", &pascal);
        std::fs::write(&target, text)?;
    }
    Ok(())
}

fn walkdir_ll(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                out.push(p.clone());
                walk(&p, out)?;
            } else {
                out.push(p);
            }
        }
        Ok(())
    }
    walk(root, &mut out)?;
    Ok(out)
}

fn to_pascal(name: &str) -> String {
    name.split(|c: char| c == '-' || c == '_' || c == '.')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn cmd_test(
    filter: Option<String>,
    jobs: Option<usize>,
    sequential: bool,
    manifest: Option<PathBuf>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = if let Some(m) = manifest {
        m.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        find_package_root(&cwd).unwrap_or(cwd)
    };
    let manifest_path = root.join("Package.ll");
    let pkg = load_package(&manifest_path)?;

    let mut lib_files = Vec::new();
    let mut test_files = Vec::new();
    // Prefer testTarget paths; also Tests/ and tests/
    for t in &pkg.targets {
        let rel = t.path.as_deref().unwrap_or(match t.kind {
            TargetKind::Test => "Tests",
            TargetKind::Library => "Sources",
            TargetKind::Executable => "Sources",
        });
        let path = root.join(rel);
        if t.kind == TargetKind::Test || t.name.to_lowercase().contains("test") {
            collect_ll_sources(&path, &mut test_files)?;
        } else if t.kind != TargetKind::Executable {
            collect_ll_sources(&path, &mut lib_files)?;
        } else {
            // executable sources — include as lib helpers unless they are pure main-only
            collect_ll_sources(&path, &mut lib_files)?;
        }
    }
    let mut seen_test_roots = std::collections::HashSet::new();
    for extra in ["Tests", "tests"] {
        let p = root.join(extra);
        if p.is_dir() {
            let key = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
            if seen_test_roots.insert(key) {
                collect_ll_sources(&p, &mut test_files)?;
            }
        }
    }
    test_files.sort();
    test_files.dedup();
    lib_files.sort();
    lib_files.dedup();
    // Don't treat the same file as both
    lib_files.retain(|f| !test_files.contains(f));

    if test_files.is_empty() {
        bail!("no test sources found under Tests/ or testTarget paths");
    }

    let mut lib_units = Vec::new();
    for f in &lib_files {
        let name = f.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == "Package.ll" || name == "Solution.ll" {
            continue;
        }
        lib_units.push((f.display().to_string(), std::fs::read_to_string(f)?));
    }
    let mut test_units = Vec::new();
    for f in &test_files {
        let name = f.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name == "Package.ll" {
            continue;
        }
        test_units.push((f.display().to_string(), std::fs::read_to_string(f)?));
    }

    let plan = test_runner::build_plan(&pkg.name, &lib_units, &test_units)?;
    if plan.cases.is_empty() {
        bail!("no @suite/@test/@Test cases discovered");
    }

    let cache = test_runner::cache_dir(&root, &plan.cache_key);
    let bin = cache.join("lang-test-bin");
    let cache_hit = bin.exists();
    if !cache_hit {
        report::status("Compiling", format!("test harness ({})", plan.cache_key));
        test_runner::write_cache_inputs(&cache, &plan)?;
        compile_test_binary(&cache, &plan, &bin)?;
    }

    test_runner::run_cases(
        &plan,
        &test_runner::RunOpts {
            filter,
            jobs,
            sequential,
            bin,
            cache_hit,
        },
    )?;
    Ok(())
}

fn compile_test_binary(cache: &Path, plan: &test_runner::TestPlan, bin: &Path) -> Result<()> {
    let mut units = plan.units.clone();
    let harness = std::fs::read_to_string(cache.join("harness.ll"))?;
    units.push((cache.join("harness.ll").display().to_string(), harness));

    let sema = lllang_sema::analyze(&units);
    let sources: std::collections::HashMap<String, String> = units.iter().cloned().collect();
    let (errs, warns) = report::emit_diagnostics(&sema.diagnostics, &sources);
    if errs > 0 {
        report::summary(errs, warns, None);
        return Err(report::exit_failure());
    }

    let program = lllang_mir::lower(&sema.module);
    let out_dir = cache.join("build");
    std::fs::create_dir_all(&out_dir)?;
    let cg = lllang_llvm::codegen(
        &program,
        &out_dir,
        &lllang_llvm::CodegenOptions {
            optimize: false,
            emit_llvm_ir: false,
        },
    )?;
    lllang_llvm::link_executable(&[cg.object_path], bin, false)?;
    Ok(())
}

fn cmd_bench() -> Result<()> {
    let timer = Timer::start();
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).unwrap_or(cwd);
    let benches = root.join("benches");
    if !benches.is_dir() {
        bail!("no benches/ directory in {}", root.display());
    }
    let mut files = Vec::new();
    collect_ll_sources(&benches, &mut files)?;
    if files.is_empty() {
        bail!("no .ll files under benches/");
    }
    report::status(
        "Running",
        format!("{} bench file{}", files.len(), if files.len() == 1 { "" } else { "s" }),
    );
    for f in &files {
        let t0 = std::time::Instant::now();
        // MVP: typecheck / analyze only; full timed run when bench packages are executables.
        let src = std::fs::read_to_string(f)?;
        let path = f.display().to_string();
        let result = lllang_sema::analyze(&[(path, src)]);
        let errs = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == lllang_syntax::Severity::Error)
            .count();
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if errs > 0 {
            report::status_err(
                "error",
                format!("{} — {errs} error(s) ({ms:.2} ms)", report::short_path(f)),
            );
        } else {
            report::status(
                "Finished",
                format!("{} ok ({ms:.2} ms check)", report::short_path(f)),
            );
        }
    }
    report::status(
        "Finished",
        format!("bench in {:.2}s", timer.elapsed_secs()),
    );
    Ok(())
}

fn cmd_check(paths: Vec<PathBuf>) -> Result<()> {
    let timer = Timer::start();
    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    };
    let mut files = Vec::new();
    for p in paths {
        collect_sources(&p, &mut files)?;
    }
    if files.is_empty() {
        bail!("no .ll or .sbl files found");
    }

    // Skip Package.ll / Solution.ll for sema (they are manifests, not programs).
    let program_files: Vec<_> = files
        .into_iter()
        .filter(|f| {
            let name = f.file_name().and_then(|s| s.to_str()).unwrap_or("");
            name != "Package.ll" && name != "Solution.ll"
        })
        .collect();

    report::status(
        "Checking",
        format!(
            "{} file{}",
            program_files.len(),
            if program_files.len() == 1 { "" } else { "s" }
        ),
    );

    let mut errors = 0usize;
    let mut warnings = 0usize;
    let mut ok_count = 0usize;
    for file in &program_files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("reading {}", file.display()))?;
        let path_str = file.display().to_string();
        let mut sources = HashMap::new();
        sources.insert(path_str.clone(), source.clone());
        let result = lllang_sema::analyze(&[(path_str, source)]);
        let (e, w) = report::emit_diagnostics(&result.diagnostics, &sources);
        errors += e;
        warnings += w;
        if e == 0 {
            ok_count += 1;
            report::check_ok(file);
        }
    }
    if errors > 0 {
        report::summary(errors, warnings, Some(timer.elapsed_secs()));
        return Err(report::exit_failure());
    }
    report::status(
        "Finished",
        format!(
            "{ok_count} file{} ok{} in {:.2}s",
            if ok_count == 1 { "" } else { "s" },
            if warnings > 0 {
                format!(", {warnings} warning{}", if warnings == 1 { "" } else { "s" })
            } else {
                String::new()
            },
            timer.elapsed_secs()
        ),
    );
    Ok(())
}

struct BuildOpts {
    release: bool,
    target: Option<String>,
    project: Option<String>,
    all: bool,
    emit_llvm: bool,
    emit_mir: bool,
    manifest: Option<PathBuf>,
    solution: Option<PathBuf>,
}

fn cmd_sdk(cmd: SdkCmd) -> Result<()> {
    let home = lllang_pack::lang_home();
    let sdk_root = home.join("sdk");
    std::fs::create_dir_all(&sdk_root)?;
    match cmd {
        SdkCmd::List => {
            report::status("Finished", format!("host {}", lllang_pack::host_triple()));
            report::status("Finished", format!("LANG_HOME {}", home.display()));
            if sdk_root.is_dir() {
                for e in std::fs::read_dir(&sdk_root)? {
                    let e = e?;
                    println!("  {}", e.file_name().to_string_lossy());
                }
            }
            Ok(())
        }
        SdkCmd::Install { version } => {
            // MVP: record requested triple/version metadata for the host.
            let triple = if version.contains('-') {
                version.clone()
            } else {
                lllang_pack::host_triple()
            };
            let dir = sdk_root.join(&triple);
            std::fs::create_dir_all(&dir)?;
            std::fs::write(
                dir.join("VERSION"),
                format!("triple={triple}\ninstalled=1\n"),
            )?;
            report::status(
                "Installed",
                format!("{triple} ({})", report::short_path(&dir)),
            );
            Ok(())
        }
        SdkCmd::Use { version } => {
            let triple = if version.contains('-') {
                version
            } else {
                lllang_pack::host_triple()
            };
            std::fs::write(sdk_root.join("current"), &triple)?;
            report::status("Finished", format!("sdk use {triple}"));
            Ok(())
        }
    }
}

fn cmd_build(opts: BuildOpts) -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let pkg_dirs = resolve_package_dirs(&cwd, &opts)?;
    let mut last_bin = None;
    for dir in pkg_dirs {
        let bin = build_package(&dir, &opts)?;
        last_bin = Some(bin);
    }
    last_bin.ok_or_else(|| anyhow::anyhow!("nothing to build"))
}

fn resolve_package_dirs(cwd: &Path, opts: &BuildOpts) -> Result<Vec<PathBuf>> {
    if let Some(ref m) = opts.manifest {
        return Ok(vec![m.parent().unwrap_or(cwd).to_path_buf()]);
    }
    let sol_path = opts
        .solution
        .clone()
        .or_else(|| find_solution_root(cwd).map(|r| r.join("Solution.ll")));
    if let Some(sol) = sol_path {
        if sol.exists() {
            let solution = load_solution(&sol)?;
            let root = sol.parent().unwrap_or(cwd);
            let mut dirs = Vec::new();
            for proj in &solution.projects {
                let p = root.join(&proj.path);
                let name = proj
                    .name
                    .clone()
                    .or_else(|| {
                        load_package(&p.join("Package.ll"))
                            .ok()
                            .map(|m| m.name)
                    })
                    .unwrap_or_default();
                let matches_project = |n: &str| {
                    n == name
                        || n == proj.path
                        || proj.path.ends_with(n)
                        || proj
                            .name
                            .as_ref()
                            .map(|pn| pn == n)
                            .unwrap_or(false)
                        || name.ends_with(&format!(".{n}"))
                        || name.ends_with(n)
                };
                if opts.all
                    || opts.project.as_ref().map(|n| matches_project(n)).unwrap_or(false)
                    || (opts.project.is_none()
                        && (solution.default_projects.is_empty()
                            || solution
                                .default_projects
                                .iter()
                                .any(|d| matches_project(d))))
                {
                    // Only build packages that produce an executable (skip pure libraries).
                    let is_exe = load_package(&p.join("Package.ll"))
                        .map(|m| {
                            m.products.iter().any(|pr| {
                                matches!(pr, lllang_manifest::Product::Executable { .. })
                            }) || m.targets.iter().any(|t| t.kind == TargetKind::Executable)
                        })
                        .unwrap_or(true);
                    if is_exe {
                        dirs.push(p);
                    }
                }
            }
            if dirs.is_empty() && !opts.all {
                // Fall back to default executable projects.
                for proj in &solution.projects {
                    let p = root.join(&proj.path);
                    let is_exe = load_package(&p.join("Package.ll"))
                        .map(|m| {
                            m.products.iter().any(|pr| {
                                matches!(pr, lllang_manifest::Product::Executable { .. })
                            }) || m.targets.iter().any(|t| t.kind == TargetKind::Executable)
                        })
                        .unwrap_or(false);
                    if is_exe {
                        dirs.push(p);
                    }
                }
            }
            return Ok(dirs);
        }
    }
    let pkg = find_package_root(cwd).context("no Package.ll or Solution.ll found")?;
    Ok(vec![pkg])
}

fn build_package(pkg_dir: &Path, opts: &BuildOpts) -> Result<PathBuf> {
    let timer = Timer::start();
    let host = lllang_pack::host_triple();
    if let Some(ref t) = opts.target {
        if t != &host {
            // Cross-target MVP: require a matching .bll in the store for locked deps;
            // native codegen still only emits the host object today.
            report::warn_msg(format!(
                "--target {t} requested; host is {host}. Linking host object; select store .bll for {t} when present."
            ));
            let lock_path = pkg_dir.join("Package.lock");
            if lock_path.exists() {
                let lock = lllang_pack::read_lock(&lock_path)?;
                for pkg in &lock.packages {
                    let has = pkg.targets.iter().any(|tg| tg.triple == *t);
                    if !has {
                        bail!(
                            "no .bll for target `{t}` in lock entry `{}` (run `lang sdk install {t}` / pack for that triple)",
                            pkg.name
                        );
                    }
                }
            }
        }
    }
    let manifest_path = pkg_dir.join("Package.ll");
    let manifest = load_package(&manifest_path)?;
    let profile = if opts.release { "release" } else { "dev" };
    report::status(
        "Compiling",
        format!("{} ({})", manifest.name, report::short_path(pkg_dir)),
    );

    let mut sources = discover_sources(pkg_dir, &manifest)?;
    // Include path-dependency packages so `import Shop.Core` resolves in the same analysis unit.
    for dep in &manifest.dependencies {
        if let lllang_manifest::PackageDep::Path { path } = dep {
            let dep_dir = pkg_dir.join(path);
            if let Ok(dep_manifest) = load_package(&dep_dir.join("Package.ll")) {
                let dep_sources = discover_sources(&dep_dir, &dep_manifest)?;
                sources.extend(dep_sources);
            } else if dep_dir.is_dir() {
                collect_sources(&dep_dir, &mut sources)?;
            }
        }
    }
    sources.sort();
    sources.dedup();
    if sources.is_empty() {
        bail!("no .ll sources in {}", pkg_dir.display());
    }

    let mut units = Vec::new();
    let mut source_map = HashMap::new();
    for s in &sources {
        let text = std::fs::read_to_string(s)?;
        let key = s.display().to_string();
        source_map.insert(key.clone(), text.clone());
        units.push((key, text));
    }
    let sema = lllang_sema::analyze(&units);
    let (err_count, warn_count) = report::emit_diagnostics(&sema.diagnostics, &source_map);
    if err_count > 0 {
        report::summary(err_count, warn_count, Some(timer.elapsed_secs()));
        return Err(report::exit_failure());
    }

    let program = lllang_mir::lower(&sema.module);
    if opts.emit_mir {
        let mir_path = pkg_dir.join(".lang/build/out.mir");
        std::fs::create_dir_all(mir_path.parent().unwrap())?;
        std::fs::write(&mir_path, lllang_mir::emit_mir(&program))?;
        report::status("Finished", format!("wrote {}", report::short_path(&mir_path)));
    }

    let cfg = if opts.release { "release" } else { "debug" };
    let out_dir = pkg_dir.join(".lang/build").join(cfg);
    std::fs::create_dir_all(&out_dir)?;

    let cg = lllang_llvm::codegen(
        &program,
        &out_dir,
        &lllang_llvm::CodegenOptions {
            optimize: opts.release,
            emit_llvm_ir: opts.emit_llvm,
        },
    )?;
    if let Some(ir) = cg.llvm_ir {
        let ir_path = out_dir.join("out.ll");
        std::fs::write(&ir_path, ir)?;
        report::status("Finished", format!("wrote {}", report::short_path(&ir_path)));
    }

    let product = manifest
        .products
        .iter()
        .find_map(|p| match p {
            lllang_manifest::Product::Executable { name, .. } => Some(name.clone()),
            _ => None,
        })
        .or_else(|| {
            manifest
                .targets
                .iter()
                .find(|t| t.kind == TargetKind::Executable)
                .map(|t| t.name.clone())
        })
        .unwrap_or_else(|| manifest.name.clone());

    let bin = out_dir.join(&product);
    report::status("Linking", report::short_path(&bin));
    lllang_llvm::link_executable(&[cg.object_path], &bin, opts.release)?;
    let warn_note = if warn_count > 0 {
        format!(
            ", {warn_count} warning{}",
            if warn_count == 1 { "" } else { "s" }
        )
    } else {
        String::new()
    };
    report::status(
        "Finished",
        format!(
            "`{profile}` profile [{}] in {:.2}s{warn_note}",
            report::short_path(&bin),
            timer.elapsed_secs()
        ),
    );
    Ok(bin)
}

fn collect_sources(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if is_source(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with('.') || name == "target" || name == "node_modules" {
                        continue;
                    }
                }
                collect_sources(&p, out)?;
            } else if is_source(&p) {
                out.push(p);
            }
        }
    }
    Ok(())
}

fn is_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("ll") | Some("sbl")
    )
}
