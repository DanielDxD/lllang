//! LLLang CLI (`lang`) — Phase 2: check (sema), build, run, lsp.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use lllang_manifest::{discover_sources, find_package_root, find_solution_root, load_package, load_solution, TargetKind};
use lllang_syntax::SourceMap;
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
        #[arg(long)]
        filter: Option<String>,
    },
    Bench,
    Check {
        paths: Vec<PathBuf>,
    },
    Fmt {
        paths: Vec<PathBuf>,
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
    Publish,
    Search { query: String },
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
    eprintln!(
        "lang: `{feature}` is planned for a later phase. See ROADMAP.md and docs/toolchain/cli-reference.md."
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
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
            project,
            all,
            emit_llvm,
            emit_mir,
            ..
        }) => {
            cmd_build(BuildOpts {
                release,
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
                project,
                all: false,
                emit_llvm: false,
                emit_mir: false,
                manifest: cli.manifest.clone(),
                solution: cli.solution.clone(),
            })?;
            let status = Command::new(&bin).args(&args).status()?;
            if !status.success() {
                bail!("program exited with {status}");
            }
            Ok(())
        }
        Some(Commands::New { list, .. }) if list => {
            println!("Templates: console, lib, cli, web, test, agent, mcp, bench, solution");
            planned("new")
        }
        Some(Commands::New { .. }) => planned("new"),
        Some(Commands::Solution { cmd }) => match cmd {
            SolutionCmd::List => planned("solution list"),
            SolutionCmd::Add { .. } => planned("solution add"),
            SolutionCmd::Remove { .. } => planned("solution remove"),
        },
        Some(Commands::Watch) => planned("watch"),
        Some(Commands::Test { .. }) => planned("test"),
        Some(Commands::Bench) => planned("bench"),
        Some(Commands::Fmt { .. }) => planned("fmt"),
        Some(Commands::Doc) => planned("doc"),
        Some(Commands::Clean) => {
            let root = std::env::current_dir()?;
            let dir = root.join(".lang");
            if dir.exists() {
                std::fs::remove_dir_all(&dir)?;
                println!("removed {}", dir.display());
            }
            Ok(())
        }
        Some(Commands::Add { .. }) => planned("add"),
        Some(Commands::Remove { .. }) => planned("remove"),
        Some(Commands::Restore) => planned("restore"),
        Some(Commands::Update) => planned("update"),
        Some(Commands::Pack { .. }) => planned("pack"),
        Some(Commands::Publish) => planned("publish"),
        Some(Commands::Search { .. }) => planned("search"),
        Some(Commands::Llm { .. }) => planned("llm"),
        Some(Commands::Sdk { .. }) => planned("sdk"),
    }
}

fn print_info() {
    println!("LLLang toolchain {VERSION}");
    println!("phase: 2 (sema + MIR + LLVM)");
    println!("host: {}", std::env::consts::OS);
    println!("arch: {}", std::env::consts::ARCH);
    if let Ok(p) = std::env::var("LLVM_SYS_221_PREFIX") {
        println!("llvm: {p}");
    } else {
        println!("llvm: set LLVM_SYS_221_PREFIX (e.g. /opt/homebrew/opt/llvm@22)");
    }
}

fn cmd_check(paths: Vec<PathBuf>) -> Result<()> {
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

    let mut errors = 0usize;
    for file in &program_files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("reading {}", file.display()))?;
        let map = SourceMap::new(&source);
        let path_str = file.display().to_string();
        let result = lllang_sema::analyze(&[(path_str, source)]);
        for d in &result.diagnostics {
            if d.severity == lllang_syntax::Severity::Error {
                errors += 1;
            }
            let loc = map.loc(d.span.start);
            eprintln!(
                "{}:{}:{}: {}: {}",
                file.display(),
                loc.line,
                loc.column,
                d.code,
                d.message
            );
        }
        if result
            .diagnostics
            .iter()
            .all(|d| d.severity != lllang_syntax::Severity::Error)
        {
            println!("{}: ok", file.display());
        }
    }
    if errors > 0 {
        bail!("{errors} error(s)");
    }
    Ok(())
}

struct BuildOpts {
    release: bool,
    project: Option<String>,
    all: bool,
    emit_llvm: bool,
    emit_mir: bool,
    manifest: Option<PathBuf>,
    solution: Option<PathBuf>,
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
    let manifest_path = pkg_dir.join("Package.ll");
    let manifest = load_package(&manifest_path)?;
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
    for s in &sources {
        let text = std::fs::read_to_string(s)?;
        units.push((s.display().to_string(), text));
    }
    let sema = lllang_sema::analyze(&units);
    let err_count = sema
        .diagnostics
        .iter()
        .filter(|d| d.severity == lllang_syntax::Severity::Error)
        .count();
    for d in &sema.diagnostics {
        eprintln!("{}: {}", d.code, d.message);
    }
    if err_count > 0 {
        bail!("sema failed with {err_count} error(s)");
    }

    let program = lllang_mir::lower(&sema.module);
    if opts.emit_mir {
        let mir_path = pkg_dir.join(".lang/build/out.mir");
        std::fs::create_dir_all(mir_path.parent().unwrap())?;
        std::fs::write(&mir_path, lllang_mir::emit_mir(&program))?;
        println!("wrote {}", mir_path.display());
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
        println!("wrote {}", ir_path.display());
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
    lllang_llvm::link_executable(&[cg.object_path], &bin, opts.release)?;
    println!("built {}", bin.display());
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
