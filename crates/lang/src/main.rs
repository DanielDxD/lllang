//! LLLang CLI (`lang`).
//!
//! Phase 1 implements: `lsp`, `check`, `--version` / `--info`.
//! Other subcommands are registered with full help and return a planned message.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use lllang_syntax::{check, SourceMap};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "lang", version = VERSION, about = "LLLang toolchain", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Print extended toolchain information
    #[arg(long)]
    info: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create a new project from a template
    New {
        template: Option<String>,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        #[arg(long)]
        list: bool,
    },
    /// Build the current project
    Build {
        #[arg(long)]
        release: bool,
        #[arg(long)]
        target: Option<String>,
    },
    /// Build and run
    Run {
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Watch and rebuild on change
    Watch,
    /// Run tests
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
    /// Run benchmarks
    Bench,
    /// Syntax-check `.ll` / `.sbl` files (Phase 1)
    Check {
        /// Paths to files or directories
        paths: Vec<PathBuf>,
    },
    /// Format sources
    Fmt {
        paths: Vec<PathBuf>,
    },
    /// Generate API documentation
    Doc,
    /// Remove build artifacts
    Clean,
    /// Add a package dependency
    Add { package: String },
    /// Remove a package dependency
    Remove { package: String },
    /// Restore dependencies from the lockfile
    Restore,
    /// Update dependencies within declared ranges
    Update,
    /// Pack a library into `.bll` / `.sbl` / `.mll`
    Pack {
        #[arg(long)]
        targets: Option<String>,
    },
    /// Publish a packed library to the registry
    Publish,
    /// Search the package registry
    Search { query: String },
    /// LLM helpers
    Llm {
        #[command(subcommand)]
        cmd: LlmCmd,
    },
    /// Manage installed SDKs
    Sdk {
        #[command(subcommand)]
        cmd: SdkCmd,
    },
    /// Start the Language Server over stdio (Phase 1)
    Lsp,
}

#[derive(Subcommand, Debug)]
enum LlmCmd {
    /// Generate `.mll` metadata
    Gen,
    /// Build an LLM context pack
    Context {
        #[arg(long)]
        budget: Option<u32>,
    },
    /// Explain a symbol
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
        Some(Commands::New { list, .. }) if list => {
            println!("Templates: console, lib, cli, web, test, agent, mcp, bench");
            planned("new")
        }
        Some(Commands::New { .. }) => planned("new"),
        Some(Commands::Build { .. }) => planned("build"),
        Some(Commands::Run { .. }) => planned("run"),
        Some(Commands::Watch) => planned("watch"),
        Some(Commands::Test { .. }) => planned("test"),
        Some(Commands::Bench) => planned("bench"),
        Some(Commands::Fmt { .. }) => planned("fmt"),
        Some(Commands::Doc) => planned("doc"),
        Some(Commands::Clean) => planned("clean"),
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
    println!("phase: 1 (syntax + LSP)");
    println!("host: {}", std::env::consts::OS);
    println!("arch: {}", std::env::consts::ARCH);
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
    let mut errors = 0usize;
    for file in files {
        let source = std::fs::read_to_string(&file)
            .with_context(|| format!("reading {}", file.display()))?;
        let diags = check(&source);
        let map = SourceMap::new(&source);
        for d in &diags {
            let loc = map.loc(d.span.start);
            eprintln!(
                "{}:{}:{}: {}: {}",
                file.display(),
                loc.line,
                loc.column,
                d.code,
                d.message
            );
            errors += 1;
        }
        if diags.is_empty() {
            println!("{}: ok", file.display());
        }
    }
    if errors > 0 {
        bail!("{errors} syntax error(s)");
    }
    Ok(())
}

fn collect_sources(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        if is_source(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?
        {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                // Skip hidden / target / node_modules
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
