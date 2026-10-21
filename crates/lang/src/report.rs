//! Colored, Cargo/Vitest-inspired terminal reporting for `lang`.

use lllang_syntax::{Diagnostic, Severity, SourceMap, Span};
use std::collections::HashMap;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;

const COLOR_AUTO: u8 = 0;
const COLOR_ALWAYS: u8 = 1;
const COLOR_NEVER: u8 = 2;

static COLOR_MODE: AtomicU8 = AtomicU8::new(COLOR_AUTO);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

impl ColorChoice {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "always" => Some(Self::Always),
            "never" => Some(Self::Never),
            _ => None,
        }
    }
}

pub fn set_color_choice(choice: ColorChoice) {
    let v = match choice {
        ColorChoice::Auto => COLOR_AUTO,
        ColorChoice::Always => COLOR_ALWAYS,
        ColorChoice::Never => COLOR_NEVER,
    };
    COLOR_MODE.store(v, Ordering::Relaxed);
}

fn colors_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    match COLOR_MODE.load(Ordering::Relaxed) {
        COLOR_ALWAYS => true,
        COLOR_NEVER => false,
        _ => io::stderr().is_terminal() || io::stdout().is_terminal(),
    }
}

struct Style;

impl Style {
    fn paint(code: &str, text: &str) -> String {
        if colors_enabled() {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn bold(text: &str) -> String {
        Self::paint("1", text)
    }
    fn dim(text: &str) -> String {
        Self::paint("2", text)
    }
    fn blue(text: &str) -> String {
        Self::paint("34", text)
    }
    fn cyan(text: &str) -> String {
        Self::paint("36", text)
    }
    fn bright_red(text: &str) -> String {
        Self::paint("1;31", text)
    }
    fn bright_yellow(text: &str) -> String {
        Self::paint("1;33", text)
    }
    fn bright_green(text: &str) -> String {
        Self::paint("1;32", text)
    }
    fn bright_cyan(text: &str) -> String {
        Self::paint("1;36", text)
    }
    fn green(text: &str) -> String {
        Self::paint("32", text)
    }
    fn red(text: &str) -> String {
        Self::paint("31", text)
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Cargo-style right-aligned status verb (12-char field).
pub fn status(verb: &str, detail: impl AsRef<str>) {
    let styled = match verb {
        "Compiling" | "Checking" | "Formatting" | "Packing" | "Restoring" | "Finished"
        | "Running" | "Linking" | "Removing" | "Adding" | "Searching" | "Installed" => {
            Style::bright_green(verb)
        }
        "error" => Style::bright_red(verb),
        "warning" => Style::bright_yellow(verb),
        "note" => Style::bright_cyan(verb),
        _ => Style::green(verb),
    };
    let plain_len = strip_ansi(&styled).chars().count();
    let pad = 12usize.saturating_sub(plain_len);
    println!("{}{} {}", " ".repeat(pad), styled, detail.as_ref());
}

pub fn status_err(verb: &str, detail: impl AsRef<str>) {
    let styled = match verb {
        "error" => Style::bright_red(verb),
        "warning" => Style::bright_yellow(verb),
        _ => Style::red(verb),
    };
    let plain_len = strip_ansi(&styled).chars().count();
    let pad = 12usize.saturating_sub(plain_len);
    eprintln!("{}{} {}", " ".repeat(pad), styled, detail.as_ref());
}

pub fn short_path(path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = path.strip_prefix(&cwd) {
            return rel.display().to_string();
        }
    }
    path.display().to_string()
}

pub fn short_str(path: &str) -> String {
    short_path(Path::new(path))
}

pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

pub fn check_ok(path: &Path) {
    let mark = Style::bright_green("✓");
    println!("  {mark} {}", Style::dim(&short_path(path)));
}

pub fn check_ok_test(label: &str) {
    let mark = Style::bright_green("✓");
    println!("  {mark} {}", Style::dim(label));
}

pub fn fmt_changed(path: &Path, check_only: bool) {
    if check_only {
        status_err(
            "warning",
            format!("{} would be reformatted", short_path(path)),
        );
    } else {
        let mark = Style::bright_cyan("~");
        println!("  {mark} {}", short_path(path));
    }
}

pub fn fmt_summary(changed: usize, total: usize, check: bool, secs: f64) {
    if check {
        if changed == 0 {
            status(
                "Finished",
                format!("fmt --check ({total} files) in {secs:.2}s"),
            );
        } else {
            status_err(
                "error",
                format!("{changed} of {total} file(s) would be reformatted"),
            );
        }
    } else if changed == 0 {
        status(
            "Finished",
            format!("already formatted ({total} files) in {secs:.2}s"),
        );
    } else {
        status(
            "Finished",
            format!("formatted {changed}/{total} file(s) in {secs:.2}s"),
        );
    }
}

/// Render diagnostics rustc-style with source snippets.
pub fn emit_diagnostics(diags: &[Diagnostic], sources: &HashMap<String, String>) -> (usize, usize) {
    let mut errors = 0usize;
    let mut warnings = 0usize;
    for d in diags {
        match d.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Note => {}
        }
        emit_one(d, sources);
    }
    (errors, warnings)
}

fn emit_one(d: &Diagnostic, sources: &HashMap<String, String>) {
    let colored_label = match d.severity {
        Severity::Error => Style::bright_red("error"),
        Severity::Warning => Style::bright_yellow("warning"),
        Severity::Note => Style::bright_cyan("note"),
    };
    eprintln!(
        "{}{}: {}",
        colored_label,
        Style::bold(&format!("[{}]", d.code)),
        Style::bold(&d.message)
    );

    let file = d
        .file
        .as_deref()
        .map(short_str)
        .unwrap_or_else(|| "<input>".into());
    let source = d
        .file
        .as_ref()
        .and_then(|f| sources.get(f).map(|s| s.as_str()))
        .or_else(|| {
            if sources.len() == 1 {
                sources.values().next().map(|s| s.as_str())
            } else {
                None
            }
        });

    if let Some(src) = source {
        let map = SourceMap::new(src);
        let (start, end) = map.span_locs(d.span);
        eprintln!(
            "  {} {}:{}:{}",
            Style::blue("-->"),
            Style::cyan(&file),
            start.line,
            start.column
        );
        print_snippet(src, &map, d.span, start.line, end.line, d.severity);
    } else {
        eprintln!("  {} {}", Style::blue("-->"), Style::cyan(&file));
    }
    eprintln!();
}

fn print_snippet(
    source: &str,
    map: &SourceMap,
    span: Span,
    start_line: u32,
    end_line: u32,
    severity: Severity,
) {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return;
    }
    let lo = (start_line as usize).saturating_sub(1).min(lines.len() - 1);
    let hi = (end_line as usize)
        .saturating_sub(1)
        .min(lines.len() - 1)
        .max(lo);
    let gutter_w = ((hi + 1) as f64).log10().floor() as usize + 1;

    eprintln!("  {}{}", " ".repeat(gutter_w), Style::blue("|"));
    for (idx, line) in lines.iter().enumerate().take(hi + 1).skip(lo) {
        let line_no = idx + 1;
        let gutter = format!("{line_no:>gutter_w$}");
        eprintln!(
            "  {} {} {}",
            Style::blue(&gutter),
            Style::blue("|"),
            line
        );

        let byte_start = map.line_start(line_no as u32);
        let byte_end = byte_start + line.len() as u32;
        if span.end <= byte_start || span.start >= byte_end + 1 {
            // Allow pointing at EOL newline.
            if !(span.start >= byte_start && span.start <= byte_end) {
                continue;
            }
        }
        let col_start = if span.start <= byte_start {
            0
        } else {
            (span.start - byte_start) as usize
        };
        let col_end = if span.end >= byte_end {
            line.chars().count().max(col_start + 1)
        } else {
            ((span.end - byte_start) as usize).max(col_start + 1)
        };
        let pad = " ".repeat(col_start);
        let carets = "^".repeat((col_end - col_start).max(1));
        let mark = match severity {
            Severity::Error => Style::bright_red(&carets),
            Severity::Warning => Style::bright_yellow(&carets),
            Severity::Note => Style::bright_cyan(&carets),
        };
        eprintln!(
            "  {} {} {}{}",
            " ".repeat(gutter_w),
            Style::blue("|"),
            pad,
            mark
        );
    }
}

pub fn summary(errors: usize, warnings: usize, secs: Option<f64>) {
    let time = secs.map(|s| format!(" in {s:.2}s")).unwrap_or_default();
    if errors > 0 {
        let msg = if warnings > 0 {
            format!(
                "could not compile due to {errors} previous error{}; {warnings} warning{}{time}",
                if errors == 1 { "" } else { "s" },
                if warnings == 1 { "" } else { "s" },
            )
        } else {
            format!(
                "could not compile due to {errors} previous error{}{time}",
                if errors == 1 { "" } else { "s" },
            )
        };
        status_err("error", msg);
    } else if warnings > 0 {
        status(
            "Finished",
            format!(
                "{warnings} warning{}{time}",
                if warnings == 1 { "" } else { "s" },
            ),
        );
    }
}

/// Marker: diagnostics / summary already printed; `main` should exit quietly.
#[derive(Debug)]
pub struct ExitFailure;

impl std::fmt::Display for ExitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit failure")
    }
}

impl std::error::Error for ExitFailure {}

pub fn exit_failure() -> anyhow::Error {
    ExitFailure.into()
}

pub fn fatal(err: &anyhow::Error) {
    if err.downcast_ref::<ExitFailure>().is_some() {
        return;
    }
    eprintln!("{}: {err:#}", Style::bright_red("error"));
}

pub fn warn_msg(msg: impl AsRef<str>) {
    status_err("warning", msg.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_parse() {
        assert_eq!(ColorChoice::parse("auto"), Some(ColorChoice::Auto));
        assert_eq!(ColorChoice::parse("never"), Some(ColorChoice::Never));
    }
}
