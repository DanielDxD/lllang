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
use std::io::BufRead;
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
        #[arg(short = 'o', long, visible_alias = "path")]
        output: Option<PathBuf>,
        #[arg(long)]
        list: bool,
        /// Skip `git init` in the generated project.
        #[arg(long)]
        no_git: bool,
        /// Use an arbitrary template directory instead of embedded templates.
        #[arg(long)]
        template_path: Option<PathBuf>,
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
        target: Option<String>,
        #[arg(long)]
        project: Option<String>,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Watch,
    /// Interactive REPL (Phase 1.6a MVP — evaluate expressions via temp package).
    Repl,
    /// DAP debug adapter stub (stdio JSON-RPC shell).
    Debug,
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
            target,
            project,
            args,
        }) => {
            let bin = cmd_build(BuildOpts {
                release,
                target,
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
            no_git,
            template_path,
        }) => {
            if list {
                cmd_new_list()
            } else {
                cmd_new(template, name, output, no_git, template_path)
            }
        }
        Some(Commands::Solution { cmd }) => match cmd {
            SolutionCmd::List => planned("solution list"),
            SolutionCmd::Add { .. } => planned("solution add"),
            SolutionCmd::Remove { .. } => planned("solution remove"),
        },
        Some(Commands::Watch) => cmd_watch(),
        Some(Commands::Repl) => cmd_repl(),
        Some(Commands::Debug) => cmd_debug_dap(),
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
        Some(Commands::Doc) => cmd_doc(),
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
        Some(Commands::Update) => cmd_update(),
        Some(Commands::Pack { targets }) => cmd_pack(targets),
        Some(Commands::Publish { registry }) => cmd_publish(registry.as_deref()),
        Some(Commands::Search { query, registry }) => cmd_search(&query, registry.as_deref()),
        Some(Commands::Llm { cmd }) => cmd_llm(cmd),
        Some(Commands::Sdk { cmd }) => cmd_sdk(cmd),
    }
}

fn print_info() {
    println!("LLLang toolchain {VERSION}");
    println!("bll-format: 2 (portable bitcode)");
    println!("host-triple: {}", lllang_pack::host_triple());
    println!("official-targets:");
    for t in lllang_pack::OFFICIAL_TRIPLES {
        println!("  {t}");
    }
    println!("host: {}", std::env::consts::OS);
    println!("arch: {}", std::env::consts::ARCH);
    if let Ok(p) = std::env::var("LLVM_SYS_221_PREFIX") {
        println!("llvm: {p}");
    } else {
        println!("llvm: set LLVM_SYS_221_PREFIX (e.g. /opt/homebrew/opt/llvm@22)");
    }
}

fn codegen_opts(optimize: bool, emit_llvm_ir: bool, triple: &str) -> lllang_llvm::CodegenOptions {
    lllang_llvm::CodegenOptions {
        optimize,
        emit_llvm_ir,
        target_triple: triple.to_string(),
    }
}

/// Materialize portable `.bll` containers into native objects for `triple`.
fn materialize_bll_objects(
    bll_paths: &[PathBuf],
    triple: &str,
    cache_root: &Path,
    optimize: bool,
) -> Result<Vec<PathBuf>> {
    let mut objs = Vec::new();
    let out_dir = cache_root.join("bll").join(triple);
    std::fs::create_dir_all(&out_dir)?;
    let ext = lllang_pack::object_ext(triple);
    for bll in bll_paths {
        let bytes = std::fs::read(bll)
            .with_context(|| format!("reading {}", bll.display()))?;
        let parsed = lllang_pack::decode_portable_bll(&bytes)?;
        let hash = lllang_pack::content_hash(&bytes);
        let short = &hash[..hash.len().min(16)];
        let obj = out_dir.join(format!("{short}.{ext}"));
        if !obj.exists() {
            lllang_llvm::materialize_bitcode(&parsed.bitcode, triple, &obj, optimize)?;
        }
        objs.push(obj);
    }
    Ok(objs)
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
    // Compile to portable LLVM bitcode (libraries need no linked executable / ll_main).
    let build_dir = root.join(".lang").join("pack");
    std::fs::create_dir_all(&build_dir)?;
    let bitcode = compile_package_bitcode(&root, &source_pairs)?;
    let out = root.join(".lang").join("dist");
    let version = "0.1.0";
    let (sbl_path, bll_path, _, _) = lllang_pack::pack_library(
        &manifest.name,
        version,
        &source_pairs,
        &bitcode,
        &out,
    )?;
    let sbl_bytes = std::fs::read(&sbl_path)?;
    let bll_bytes = std::fs::read(&bll_path)?;
    // Lock hashes are of on-disk bytes (sbl includes integrity footer).
    let sbl_hash = lllang_pack::content_hash(&sbl_bytes);
    let bll_hash = lllang_pack::content_hash(&bll_bytes);
    let entry = lllang_pack::install_to_store(&manifest.name, version, &sbl_bytes, &bll_bytes)?;
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
        version: version.into(),
        sbl: sbl_hash,
        bll: bll_hash.clone(),
        source: format!("path:{}", root.display()),
        targets: vec![lllang_pack::LockedTarget {
            triple: lllang_pack::PORTABLE_TRIPLE.into(),
            bll: bll_hash,
        }],
    });
    lllang_pack::write_lock(&lock_path, &lock)?;
    report::status("Packing", format!("{} -> {}", manifest.name, report::short_path(&entry)));
    report::status("Finished", format!("sbl {}", report::short_path(&sbl_path)));
    report::status("Finished", format!("bll {}", report::short_path(&bll_path)));
    Ok(())
}

/// Sema + MIR + LLVM bitcode for a package (no link). Used by `lang pack`.
fn compile_package_bitcode(root: &Path, source_pairs: &[(String, String)]) -> Result<Vec<u8>> {
    let mut units = source_pairs.to_vec();
    if units.is_empty() {
        bail!("no sources to pack in {}", root.display());
    }
    let source_map: HashMap<String, String> = units.iter().cloned().collect();
    let sema = lllang_sema::analyze(&units);
    let (err_count, warn_count) = report::emit_diagnostics(&sema.diagnostics, &source_map);
    if err_count > 0 {
        report::summary(err_count, warn_count, None);
        return Err(report::exit_failure());
    }
    let program = lllang_mir::lower(&sema.module);
    // Target triple is rebound at consumer materialization; host is fine for IR shape.
    let triple = lllang_pack::resolve_build_target(None)
        .unwrap_or_else(|_| lllang_pack::host_triple());
    let (_ir, bitcode) = lllang_llvm::codegen_bitcode(
        &program,
        &codegen_opts(true, false, &triple),
    )?;
    Ok(bitcode)
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
    let (name, version) = if let Some((n, v)) = package.split_once('@') {
        (n.to_string(), v.to_string())
    } else {
        (package.to_string(), "0.1.0".into())
    };
    let manifest_path = root.join("Package.ll");
    if !manifest_path.exists() {
        bail!("no Package.ll in {}", root.display());
    }
    append_registry_dep_to_package_ll(&manifest_path, &name, &version)?;
    let locked = lllang_pack::resolve_locked_package(&name, &version)?;
    let lock_path = root.join("Package.lock");
    let mut lock = if lock_path.exists() {
        lllang_pack::read_lock(&lock_path).unwrap_or_default()
    } else {
        lllang_pack::LockFile {
            version: 1,
            packages: vec![],
        }
    };
    lock.packages.retain(|p| p.name != name);
    lock.packages.push(locked);
    lllang_pack::write_lock(&lock_path, &lock)?;
    report::status(
        "Adding",
        format!("{name}@{version} -> Package.ll + Package.lock"),
    );
    Ok(())
}

/// Append `.package(id: "...", from: "...")` to the root Package.ll dependencies array.
fn append_registry_dep_to_package_ll(path: &Path, id: &str, version: &str) -> Result<()> {
    let text = std::fs::read_to_string(path)?;
    let needle = format!("id: \"{id}\"");
    if text.contains(&needle) {
        report::status("Finished", format!("Package.ll already lists `{id}`"));
        return Ok(());
    }
    let entry = format!(".package(id: \"{id}\", from: \"{version}\")");
    let updated = if let Some(idx) = text.find("dependencies:") {
        let after = &text[idx..];
        if let Some(rel_open) = after.find('[') {
            let open = idx + rel_open;
            let close_rel = after[rel_open..]
                .find(']')
                .context("Package.ll: dependencies array missing `]`")?;
            let close = open + close_rel;
            let inner = text[open + 1..close].trim();
            let insertion = if inner.is_empty() {
                format!("\n        {entry},\n    ")
            } else {
                let needs_comma = !inner.trim_end().ends_with(',');
                let comma = if needs_comma { "," } else { "" };
                format!("{comma}\n        {entry},\n    ")
            };
            let mut out = String::with_capacity(text.len() + insertion.len() + 8);
            out.push_str(&text[..close]);
            out.push_str(&insertion);
            out.push_str(&text[close..]);
            out
        } else {
            bail!("Package.ll: could not find dependencies array");
        }
    } else if let Some(idx) = text.find("targets:") {
        // Insert a dependencies: [...] block before targets.
        let block = format!(
            "    dependencies: [\n        {entry},\n    ],\n"
        );
        let mut out = String::with_capacity(text.len() + block.len());
        out.push_str(&text[..idx]);
        out.push_str(&block);
        out.push_str(&text[idx..]);
        out
    } else {
        bail!("Package.ll: no `dependencies:` or `targets:` to edit");
    };
    std::fs::write(path, updated)?;
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
    let version = "0.1.0";
    let dist = root.join(".lang").join("dist");
    let sbl_path = dist.join("package.sbl");
    let bll_path = dist.join("package.bll");
    // Prefer artifacts from `lang pack`; otherwise pack now (do not publish empty stubs).
    let (sbl_bytes, bll_bytes) = if sbl_path.exists() && bll_path.exists() {
        (std::fs::read(&sbl_path)?, std::fs::read(&bll_path)?)
    } else {
        report::status("Packing", "no .lang/dist artifacts; running pack");
        cmd_pack(None)?;
        (std::fs::read(&sbl_path)?, std::fs::read(&bll_path)?)
    };
    if bll_bytes.is_empty() {
        bail!("refusing to publish empty .bll; run `lang pack` first");
    }
    if !lllang_pack::is_portable_bll(&bll_bytes) {
        bail!("refusing to publish non-portable .bll; run `lang pack` (BLL v2)");
    }
    let reg_root = lllang_pack::registry_root(registry);
    let reg = lllang_pack::FileRegistry::open(&reg_root)?;
    let pkg = reg.publish(&manifest.name, version, &sbl_bytes, &bll_bytes)?;
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

static EMBEDDED_TEMPLATES: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../templates");

fn templates_disk_dir() -> PathBuf {
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
    no_git: bool,
    template_path: Option<PathBuf>,
) -> Result<()> {
    let pascal_name;
    let package;
    let dest;
    let template_label;

    if let Some(custom) = template_path {
        if !custom.is_dir() {
            bail!("--template-path must be a directory: {}", custom.display());
        }
        let name = name.unwrap_or_else(|| {
            custom
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("App")
                .to_string()
        });
        dest = output.unwrap_or_else(|| PathBuf::from(&name));
        if dest.exists() {
            bail!("{} already exists", dest.display());
        }
        std::fs::create_dir_all(&dest)?;
        pascal_name = to_pascal(&name);
        package = to_package_name(&name);
        template_label = custom.display().to_string();
        copy_template_disk(&custom, &dest, &name, &pascal_name, &package)?;
        write_skeleton(&dest, &name, "custom")?;
        if !no_git {
            let _ = Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&dest)
                .status();
        }
        report::status(
            "Finished",
            format!("created {} from `--template-path`", dest.display()),
        );
        return Ok(());
    }

    let template = template.unwrap_or_else(|| "console".into());
    template_label = template.clone();
    if !TEMPLATES.contains(&template.as_str()) {
        bail!("unknown template `{template}`; use --list");
    }
    let name = name.unwrap_or_else(|| template.clone());
    dest = output.unwrap_or_else(|| PathBuf::from(&name));
    if dest.exists() {
        bail!("{} already exists", dest.display());
    }
    std::fs::create_dir_all(&dest)?;
    pascal_name = to_pascal(&name);
    package = to_package_name(&name);

    // Prefer on-disk templates when developing in-repo; always fall back to embed.
    let disk = templates_disk_dir().join(&template);
    if disk.is_dir() {
        copy_template_disk(&disk, &dest, &name, &pascal_name, &package)?;
    } else {
        let dir = EMBEDDED_TEMPLATES
            .get_dir(&template)
            .ok_or_else(|| anyhow::anyhow!("embedded template `{template}` missing"))?;
        copy_template_embed(dir, &dest, &name, &pascal_name, &package)?;
    }
    write_skeleton(&dest, &name, &template_label)?;
    if !no_git {
        let _ = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&dest)
            .status();
    }
    report::status(
        "Finished",
        format!(
            "created {} from template `{template_label}`",
            dest.display()
        ),
    );
    Ok(())
}

fn cmd_repl() -> Result<()> {
    use std::io::{self, BufRead, Write};
    println!("LLLang REPL (MVP). Type expressions; `:quit` to exit.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        write!(stdout, "ll> ")?;
        stdout.flush()?;
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == ":quit" || line == ":q" {
            break;
        }
        let tmp = std::env::temp_dir().join(format!("lllang-repl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src"))?;
        std::fs::write(
            tmp.join("Package.ll"),
            r#"// lang-tools-version: 0.1
import Std.PackageDescription
let package = Package(
    name: "repl",
    products: [.executable(name: "repl", targets: ["Main"])],
    targets: [.executableTarget(name: "Main", path: "src")],
)
"#,
        )?;
        let src = format!(
            "namespace Repl\nimport Std.IO\npublic func main() {{\n    print(\"({line})\")\n}}\n"
        );
        std::fs::write(tmp.join("src/main.ll"), src)?;
        let status = Command::new(std::env::current_exe()?)
            .arg("run")
            .current_dir(&tmp)
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => eprintln!("(exit {s})"),
            Err(e) => eprintln!("repl error: {e}"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
    Ok(())
}

fn cmd_debug_dap() -> Result<()> {
    report::status(
        "Finished",
        "DAP adapter MVP: stdio JSON-RPC shell not yet speaking full DAP — use `lang check`/`lldb` for now",
    );
    // Accept one initialize-like line then exit so tooling can probe.
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    println!(
        "{{\"type\":\"response\",\"request_seq\":1,\"success\":true,\"command\":\"initialize\",\"body\":{{\"supportsConfigurationDoneRequest\":false}}}}"
    );
    Ok(())
}

fn cmd_doc() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let out = cwd.join(".lang/doc");
    std::fs::create_dir_all(&out)?;
    let index = out.join("index.html");
    std::fs::write(
        &index,
        "<!DOCTYPE html><html><body><h1>LLLang docs</h1><p>Generated by <code>lang doc</code> (MVP stub).</p></body></html>\n",
    )?;
    report::status("Finished", format!("wrote {}", report::short_path(&index)));
    Ok(())
}

fn cmd_update() -> Result<()> {
    report::status(
        "Finished",
        "update: re-ran restore against Package.lock (constraint bumping MVP defers to lock refresh)",
    );
    cmd_restore()
}

fn cmd_llm(cmd: LlmCmd) -> Result<()> {
    match cmd {
        LlmCmd::Gen => {
            report::status("Finished", "llm gen: refresh package.mll via `lang llm context`");
            Ok(())
        }
        LlmCmd::Context { budget } => {
            let _ = budget;
            let cwd = std::env::current_dir()?;
            let mll = cwd.join("package.mll");
            if !mll.exists() {
                std::fs::write(
                    &mll,
                    "package: repl\nsummary: auto-generated context stub\n",
                )?;
            }
            report::status("Finished", format!("wrote {}", report::short_path(&mll)));
            Ok(())
        }
        LlmCmd::Explain { symbol } => {
            println!("symbol `{symbol}`: see `lang check` / LSP hover (explain MVP)");
            Ok(())
        }
    }
}

fn to_package_name(name: &str) -> String {
    name.replace('-', ".")
}

fn expand_placeholders(text: &str, name: &str, pascal: &str, package: &str) -> String {
    text.replace("{{name}}", name)
        .replace("{{Name}}", pascal)
        .replace("{{package}}", package)
}

fn expand_path(rel: &str, name: &str, pascal: &str, package: &str) -> String {
    let tests = format!("{pascal}Tests");
    let src_pascal = format!("Sources/{pascal}");
    let tests_pascal = format!("Tests/{pascal}");
    let mut s = rel.to_string();
    // Solution members keep fixed `Core` / `Cli` directory names.
    let in_solution_member = rel.contains("apps/") || rel.contains("packages/");
    let mut renames: Vec<(&str, &str)> = vec![
        ("MyBenchTests", tests.as_str()),
        ("LibTests", tests.as_str()),
        ("AppTests", tests.as_str()),
        ("CliTests", tests.as_str()),
        ("WebTests", tests.as_str()),
        ("MyCliTests", tests.as_str()),
        ("MyWebTests", tests.as_str()),
        ("MyAgent", pascal),
        ("MyMcp", pascal),
        ("MyBench", pascal),
        ("MyCli", pascal),
        ("MyWeb", pascal),
        ("Hello", pascal),
        ("Sources/Lib", src_pascal.as_str()),
        ("Sources/Web", src_pascal.as_str()),
        ("Sources/App", src_pascal.as_str()),
        ("Tests/Lib", tests_pascal.as_str()),
        ("Tests/App", tests_pascal.as_str()),
        ("Tests/Web", tests_pascal.as_str()),
    ];
    if !in_solution_member {
        renames.push(("Sources/Cli", src_pascal.as_str()));
        renames.push(("Tests/Cli", tests_pascal.as_str()));
    }
    for (from, to) in renames {
        s = s.replace(from, to);
    }
    expand_placeholders(&s, name, pascal, package)
}

fn copy_template_disk(
    src: &Path,
    dest: &Path,
    name: &str,
    pascal: &str,
    package: &str,
) -> Result<()> {
    for entry in walkdir_files(src)? {
        let rel = entry.strip_prefix(src).unwrap();
        let rel_s = rel.to_string_lossy();
        // Skeleton files are written by write_skeleton; skip duplicates from template.
        if is_skeleton_path(&rel_s) {
            continue;
        }
        let target = dest.join(expand_path(&rel_s, name, pascal, package));
        if entry.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = std::fs::read_to_string(&entry).unwrap_or_default();
        std::fs::write(
            &target,
            expand_placeholders(&text, name, pascal, package),
        )?;
    }
    Ok(())
}

fn copy_template_embed(
    dir: &include_dir::Dir,
    dest: &Path,
    name: &str,
    pascal: &str,
    package: &str,
) -> Result<()> {
    fn walk(
        dir: &include_dir::Dir,
        base: &str,
        dest: &Path,
        name: &str,
        pascal: &str,
        package: &str,
    ) -> Result<()> {
        for file in dir.files() {
            let path = file.path().to_string_lossy();
            let rel = path
                .strip_prefix(base)
                .unwrap_or(path.as_ref())
                .trim_start_matches('/');
            if is_skeleton_path(rel) {
                continue;
            }
            let target = dest.join(expand_path(rel, name, pascal, package));
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let text = file.contents_utf8().unwrap_or("");
            std::fs::write(
                &target,
                expand_placeholders(text, name, pascal, package),
            )?;
        }
        for sub in dir.dirs() {
            walk(sub, base, dest, name, pascal, package)?;
        }
        Ok(())
    }
    let base = dir.path().to_string_lossy().to_string();
    walk(dir, &base, dest, name, pascal, package)
}

fn is_skeleton_path(rel: &str) -> bool {
    matches!(
        rel,
        ".gitignore"
            | ".editorconfig"
            | "README.md"
            | ".github/workflows/ci.yml"
    ) || rel.ends_with("/.gitignore")
        || rel.ends_with("/.editorconfig")
        || rel.ends_with("/README.md")
        || rel.ends_with("/.github/workflows/ci.yml")
}

fn write_skeleton(dest: &Path, name: &str, template: &str) -> Result<()> {
    let gitignore = "\
.lang/
*.o
*.ll.bc
.DS_Store
";
    let editorconfig = "\
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 4
trim_trailing_whitespace = true
";
    let has_tests = matches!(
        template,
        "lib" | "test" | "cli" | "web" | "bench"
    );
    let has_bench = template == "bench";
    let mut readme = format!(
        "# {name}\n\nGenerated by `lang new {template}`.\n\n```bash\nlang check\nlang build\n"
    );
    if matches!(
        template,
        "console" | "cli" | "web" | "agent" | "mcp" | "solution"
    ) {
        readme.push_str("lang run\n");
    }
    if has_tests {
        readme.push_str("lang test\n");
    }
    if has_bench {
        readme.push_str("lang bench\n");
    }
    readme.push_str("```\n");

    let ci_test = if has_tests {
        "      - run: lang test\n"
    } else if matches!(template, "console" | "cli" | "web" | "agent" | "mcp") {
        "      - run: lang build\n      - run: lang run\n"
    } else if template == "solution" {
        "      - run: lang build\n"
    } else {
        "      - run: lang check\n"
    };
    let ci = format!(
        "\
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install LLLang
        run: echo \"Install lang toolchain for CI\"
      - run: lang check
{ci_test}"
    );

    std::fs::write(dest.join(".gitignore"), gitignore)?;
    std::fs::write(dest.join(".editorconfig"), editorconfig)?;
    std::fs::write(dest.join("README.md"), readme)?;
    let wf = dest.join(".github/workflows");
    std::fs::create_dir_all(&wf)?;
    std::fs::write(wf.join("ci.yml"), ci)?;
    Ok(())
}

fn walkdir_files(root: &Path) -> Result<Vec<PathBuf>> {
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
    let triple = lllang_pack::resolve_build_target(None)?;
    let cg = lllang_llvm::codegen(
        &program,
        &out_dir,
        &codegen_opts(false, false, &triple),
    )?;
    lllang_llvm::link_executable(&[cg.object_path], bin, false, &triple)?;
    Ok(())
}

fn cmd_bench() -> Result<()> {
    let timer = Timer::start();
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).unwrap_or(cwd);
    let benches_dir = root.join("benches");
    if !benches_dir.is_dir() {
        bail!("no benches/ directory in {}", root.display());
    }
    let mut bench_files = Vec::new();
    collect_ll_sources(&benches_dir, &mut bench_files)?;
    if bench_files.is_empty() {
        bail!("no .ll files under benches/");
    }

    let mut bench_fns: Vec<(String, String)> = Vec::new(); // (namespace, func)
    for f in &bench_files {
        let src = std::fs::read_to_string(f)?;
        let parsed = lllang_syntax::parse(&src);
        let mut ns = String::new();
        for d in &parsed.file.decls {
            match d {
                lllang_syntax::Decl::Namespace(n) => {
                    ns = n.path.segments.join(".");
                }
                lllang_syntax::Decl::Func(func) => {
                    let is_bench = func.attrs.iter().any(|a| {
                        a.name.eq_ignore_ascii_case("Bench") || a.name.eq_ignore_ascii_case("bench")
                    });
                    if is_bench {
                        bench_fns.push((ns.clone(), func.name.clone()));
                    }
                }
                _ => {}
            }
        }
    }
    if bench_fns.is_empty() {
        bail!("no @Bench functions found under benches/");
    }

    let manifest = load_package(&root.join("Package.ll"))?;
    let mut lib_sources = discover_sources(&root, &manifest)?;
    lib_sources.retain(|p| {
        let s = p.to_string_lossy();
        !s.contains("/Tests/") && !s.contains("/benches/")
    });

    let mut units: Vec<(String, String)> = Vec::new();
    for s in &lib_sources {
        units.push((s.display().to_string(), std::fs::read_to_string(s)?));
    }
    for f in &bench_files {
        units.push((f.display().to_string(), std::fs::read_to_string(f)?));
    }

    let mut harness = String::from(
        "namespace BenchHarness\n\nimport Std.IO\n\n",
    );
    let mut imported = std::collections::HashSet::new();
    for (ns, _) in &bench_fns {
        if !ns.is_empty() && imported.insert(ns.clone()) {
            harness.push_str(&format!("import {ns}\n"));
        }
    }
    harness.push_str("\npublic func main() {\n");
    const ITERS: i64 = 200;
    for (ns, name) in &bench_fns {
        let _ = ns;
        harness.push_str(&format!(
            "    let t0_{name} = Instant.now()\n\
             \x20   var i_{name} = 0\n\
             \x20   while i_{name} < {ITERS} {{\n\
             \x20       {name}()\n\
             \x20       i_{name} = i_{name} + 1\n\
             \x20   }}\n\
             \x20   let t1_{name} = Instant.now()\n\
             \x20   print(\"{name}: \\(t1_{name} - t0_{name}) ms ({ITERS} iters)\")\n"
        ));
    }
    harness.push_str("}\n");

    let cache = root.join(".lang/bench");
    std::fs::create_dir_all(&cache)?;
    let harness_path = cache.join("harness.ll");
    std::fs::write(&harness_path, &harness)?;
    units.push((harness_path.display().to_string(), harness));

    report::status(
        "Running",
        format!(
            "{} bench{}",
            bench_fns.len(),
            if bench_fns.len() == 1 { "" } else { "es" }
        ),
    );

    let source_map: HashMap<String, String> = units.iter().cloned().collect();
    let sema = lllang_sema::analyze(&units);
    let (errs, warns) = report::emit_diagnostics(&sema.diagnostics, &source_map);
    if errs > 0 {
        report::summary(errs, warns, Some(timer.elapsed_secs()));
        return Err(report::exit_failure());
    }
    let program = lllang_mir::lower(&sema.module);
    let out_dir = cache.join("build");
    std::fs::create_dir_all(&out_dir)?;
    let triple = lllang_pack::resolve_build_target(None)?;
    let cg = lllang_llvm::codegen(
        &program,
        &out_dir,
        &codegen_opts(true, false, &triple),
    )?;
    let bin = cache.join("lang-bench-bin");
    lllang_llvm::link_executable(&[cg.object_path], &bin, true, &triple)?;

    let output = Command::new(&bin).current_dir(&root).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!("bench binary failed\n{stdout}{stderr}");
    }
    for line in stdout.lines() {
        report::status("Finished", line);
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

    // Analyze the package as one unit so cross-file imports resolve (templates / libs).
    let mut units = Vec::new();
    let mut source_map = HashMap::new();
    for file in &program_files {
        // Skip benches/ and harness caches during normal check.
        let s = file.to_string_lossy();
        if s.contains("/.lang/") {
            continue;
        }
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("reading {}", file.display()))?;
        let path_str = file.display().to_string();
        source_map.insert(path_str.clone(), source.clone());
        units.push((path_str, source));
    }
    let result = lllang_sema::analyze(&units);
    let (errors, warnings) = report::emit_diagnostics(&result.diagnostics, &source_map);
    if errors > 0 {
        report::summary(errors, warnings, Some(timer.elapsed_secs()));
        return Err(report::exit_failure());
    }
    for file in &program_files {
        let s = file.to_string_lossy();
        if s.contains("/.lang/") {
            continue;
        }
        report::check_ok(file);
    }
    let ok_count = units.len();
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
    let triple = lllang_pack::resolve_build_target(opts.target.as_deref())?;
    if triple != host {
        // Cross-link needs a runtime (and often a sysroot) for the triple.
        let rt = lllang_llvm::runtime_lib_path_for(&triple);
        if !rt.exists() && !lllang_pack::sdk_triple_installed(&triple) {
            bail!(
                "no runtime/SDK for target `{triple}` (host is {host}); install runtime under {} \
                 or `lang sdk install {triple}`",
                lllang_pack::lang_home()
                    .join("toolchain")
                    .join("lib")
                    .join(&triple)
                    .display()
            );
        }
    }
    let manifest_path = pkg_dir.join("Package.ll");
    let manifest = load_package(&manifest_path)?;
    let profile = if opts.release { "release" } else { "dev" };
    // Drive `$DEBUG` / `$RELEASE` for compile-time constants.
    if opts.release {
        std::env::set_var("LANG_RELEASE", "1");
    } else {
        std::env::remove_var("LANG_RELEASE");
    }
    report::status(
        "Compiling",
        format!(
            "{} ({}) [{}]",
            manifest.name,
            report::short_path(pkg_dir),
            triple
        ),
    );

    let (units, bll_paths) = collect_analysis_units(pkg_dir, &manifest)?;
    if units.is_empty() {
        bail!("no .ll sources in {}", pkg_dir.display());
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

    let cfg = if opts.release { "release" } else { "debug" };
    let out_dir = pkg_dir.join(".lang/build").join(cfg).join(&triple);
    std::fs::create_dir_all(&out_dir)?;
    let bin = out_dir.join(&product);
    let ext = lllang_pack::object_ext(&triple);
    let object_path = out_dir.join(format!("out.{ext}"));

    let fingerprint = build_fingerprint(&units, &bll_paths, opts.release, opts.emit_llvm, &triple);
    let cache_dir = pkg_dir.join(".lang/cache/build").join(cfg).join(&triple);
    std::fs::create_dir_all(&cache_dir)?;
    let fp_path = cache_dir.join("fingerprint");
    let cached_fp = std::fs::read_to_string(&fp_path).unwrap_or_default();
    let units_unchanged = cached_fp.trim() == fingerprint && object_path.exists() && bin.exists();

    // Skip LLVM emit when all units + deps are unchanged (still require a prior object).
    if units_unchanged && !opts.emit_mir {
        report::status(
            "Finished",
            format!(
                "`{profile}` profile [{}] (incremental, unchanged) in {:.2}s",
                report::short_path(&bin),
                timer.elapsed_secs()
            ),
        );
        return Ok(bin);
    }

    let mut source_map = HashMap::new();
    for (k, v) in &units {
        source_map.insert(k.clone(), v.clone());
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

    let cg = lllang_llvm::codegen(
        &program,
        &out_dir,
        &codegen_opts(opts.release, opts.emit_llvm, &triple),
    )?;
    if let Some(ir) = cg.llvm_ir {
        let ir_path = out_dir.join("out.ll");
        std::fs::write(&ir_path, ir)?;
        report::status("Finished", format!("wrote {}", report::short_path(&ir_path)));
    }

    let bll_objs = materialize_bll_objects(
        &bll_paths,
        &triple,
        &pkg_dir.join(".lang/cache"),
        opts.release,
    )?;

    // Always re-link when units or deps changed (this path).
    report::status("Linking", report::short_path(&bin));
    let mut link_objs = vec![cg.object_path];
    link_objs.extend(bll_objs);
    // Phase 1.2: `@interop(.c)` — compile adjacent `.c` under `interop/` or listed in sources.
    link_objs.extend(compile_interop_c(pkg_dir, &out_dir)?);
    lllang_llvm::link_executable(&link_objs, &bin, opts.release, &triple)?;
    std::fs::write(&fp_path, &fingerprint)?;
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

fn build_fingerprint(
    units: &[(String, String)],
    bll_objs: &[PathBuf],
    release: bool,
    emit_llvm: bool,
    triple: &str,
) -> String {
    let mut acc = String::new();
    acc.push_str(&format!(
        "release={release};emit_llvm={emit_llvm};triple={triple}\n"
    ));
    let mut sorted: Vec<_> = units.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, text) in sorted {
        acc.push_str(path);
        acc.push('\t');
        acc.push_str(&lllang_pack::content_hash(text.as_bytes()));
        acc.push('\n');
    }
    for bll in bll_objs {
        let h = std::fs::read(bll)
            .map(|b| lllang_pack::content_hash(&b))
            .unwrap_or_else(|_| "missing".into());
        acc.push_str(&format!("bll:{}\t{h}\n", bll.display()));
    }
    lllang_pack::content_hash(acc.as_bytes())
}

fn cmd_watch() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = find_package_root(&cwd).context("no Package.ll found")?;
    report::status(
        "Watching",
        format!("{} (Ctrl-C to stop)", report::short_path(&root)),
    );
    let mut last = watch_fingerprint(&root)?;
    // Initial build.
    let _ = cmd_build(BuildOpts {
        release: false,
        target: None,
        project: None,
        all: false,
        emit_llvm: false,
        emit_mir: false,
        manifest: None,
        solution: None,
    });
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let now = match watch_fingerprint(&root) {
            Ok(f) => f,
            Err(_) => continue,
        };
        if now == last {
            continue;
        }
        last = now;
        report::status("Rebuilding", "source change detected");
        if let Err(e) = cmd_build(BuildOpts {
            release: false,
            target: None,
            project: None,
            all: false,
            emit_llvm: false,
            emit_mir: false,
            manifest: None,
            solution: None,
        }) {
            report::status_err("error", format!("{e:#}"));
        }
    }
}

fn watch_fingerprint(root: &Path) -> Result<u64> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    let mut paths = Vec::new();
    let pkg = root.join("Package.ll");
    if pkg.exists() {
        paths.push(pkg);
    }
    let lock = root.join("Package.lock");
    if lock.exists() {
        paths.push(lock);
    }
    collect_sources(root, &mut paths)?;
    paths.sort();
    for p in paths {
        p.hash(&mut hasher);
        if let Ok(meta) = std::fs::metadata(&p) {
            if let Ok(modified) = meta.modified() {
                modified.hash(&mut hasher);
            }
            meta.len().hash(&mut hasher);
        }
    }
    Ok(hasher.finish())
}

/// Local sources + path deps (as `.ll`) + store registry deps (as `.sbl` stubs) and `.bll` paths.
fn collect_analysis_units(
    pkg_dir: &Path,
    manifest: &lllang_manifest::PackageManifest,
) -> Result<(Vec<(String, String)>, Vec<PathBuf>)> {
    let mut sources = discover_sources(pkg_dir, manifest)?;
    let mut registry_deps: Vec<(String, String)> = Vec::new();
    for dep in &manifest.dependencies {
        match dep {
            lllang_manifest::PackageDep::Path { path } => {
                let dep_dir = pkg_dir.join(path);
                if let Ok(dep_manifest) = load_package(&dep_dir.join("Package.ll")) {
                    let dep_sources = discover_sources(&dep_dir, &dep_manifest)?;
                    sources.extend(dep_sources);
                } else if dep_dir.is_dir() {
                    collect_sources(&dep_dir, &mut sources)?;
                }
            }
            lllang_manifest::PackageDep::Registry { id, from } => {
                let version = from.clone().unwrap_or_else(|| "0.1.0".into());
                registry_deps.push((id.clone(), version));
            }
        }
    }
    sources.sort();
    sources.dedup();

    let mut units = Vec::new();
    for s in &sources {
        let text = std::fs::read_to_string(s)?;
        units.push((s.display().to_string(), text));
    }

    let lock_path = pkg_dir.join("Package.lock");
    let lock = if lock_path.exists() {
        Some(lllang_pack::read_lock(&lock_path)?)
    } else {
        None
    };
    if let Some(ref l) = lock {
        lllang_pack::ensure_restored(l)?;
    }
    let (sbl_units, bll_objs) =
        lllang_pack::resolve_prebuilt_deps(lock.as_ref(), &registry_deps)?;
    units.extend(sbl_units);
    Ok((units, bll_objs))
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

/// Compile `interop/**/*.c` (and `src/**/*.c`) with host clang for `@interop(.c)` linking.
fn compile_interop_c(pkg_dir: &Path, out_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut c_files = Vec::new();
    for sub in ["interop", "src"] {
        let root = pkg_dir.join(sub);
        if root.is_dir() {
            collect_c_sources(&root, &mut c_files)?;
        }
    }
    let mut objs = Vec::new();
    for c in c_files {
        let stem = c
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("interop");
        let obj = out_dir.join(format!("interop_{stem}.o"));
        let status = Command::new("clang")
            .args(["-c", "-std=c11", "-O1"])
            .arg(&c)
            .arg("-o")
            .arg(&obj)
            .status()
            .with_context(|| format!("clang compile {}", c.display()))?;
        if !status.success() {
            bail!("failed to compile interop C {}", c.display());
        }
        objs.push(obj);
    }
    Ok(objs)
}

fn collect_c_sources(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("c") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let p = entry?.path();
            if p.is_dir() {
                collect_c_sources(&p, out)?;
            } else if p.extension().and_then(|e| e.to_str()) == Some("c") {
                out.push(p);
            }
        }
    }
    Ok(())
}
