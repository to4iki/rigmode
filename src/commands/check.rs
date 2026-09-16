use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::ValueEnum;

use crate::adapters;
use crate::cli::Agent;
use crate::config::Config;
use crate::mode::{self, Mode};

const MAX_BODY_CHARS: usize = 10_000;

pub fn execute(modes_dirs: Vec<PathBuf>, config: &Config) -> Result<()> {
    let dirs = config.resolve_modes_dirs(modes_dirs)?;
    let mut warnings = 0;
    let mut errors = 0;

    println!("modes_dirs:");
    for dir in &dirs {
        let status = if dir.is_dir() { "ok" } else { "missing" };
        println!("  [{status}] {}", dir.display());
        if !dir.is_dir() {
            warnings += 1;
        }
    }

    let paths = mode::list_mode_paths(&dirs)?;
    let mut seen = std::collections::HashSet::new();

    if paths.is_empty() {
        println!("modes: none found");
        warnings += 1;
    } else {
        println!("modes: {} file(s)", paths.len());
    }

    for path in &paths {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                println!("  ! {}: error: {e}", path.display());
                errors += 1;
                continue;
            }
        };
        let m = mode::parse_mode(&text, path);
        for issue in validate_mode(&m) {
            println!("  ! {}: {issue}", m.name);
            if issue.starts_with("error:") {
                errors += 1;
            } else {
                warnings += 1;
            }
        }
        let stem = m.path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem != m.name {
            println!(
                "  ! {}: warning: name {:?} does not match filename stem {:?}",
                m.name, m.name, stem
            );
            warnings += 1;
        }
        if !seen.insert(m.name.clone()) {
            println!(
                "  ! {}: warning: duplicate name, earlier directory wins at runtime",
                m.name
            );
            warnings += 1;
        }
    }

    let mut registered = 0;
    // Driven off the enum so a new agent cannot be added without being checked.
    for agent in Agent::value_variants() {
        let label = agent.as_str();
        match adapters::hooks(*agent).registered() {
            Ok(Some(reg)) => {
                registered += 1;
                match reg.binary {
                    Some(bin) if Path::new(&bin).is_file() => {
                        println!("hook[{label}]: registered -> {bin} [ok]")
                    }
                    Some(bin) => {
                        println!("hook[{label}]: registered -> {bin} [missing binary]");
                        errors += 1;
                    }
                    None => println!(
                        "hook[{label}]: registered -> {} [unverified command]",
                        reg.command
                    ),
                }
            }
            Ok(None) => println!("hook[{label}]: not registered"),
            // An unreadable hook file is the user's to fix; it is not a reason
            // to abandon the rest of the report.
            Err(e) => {
                println!("hook[{label}]: error: {e:#}");
                errors += 1;
            }
        }
    }
    // One agent is enough; a Claude-only or Codex-only setup is not a warning.
    if registered == 0 {
        warnings += 1;
    }

    if errors > 0 {
        bail!("{errors} error(s), {warnings} warning(s)");
    }
    println!("check passed ({warnings} warning(s))");
    Ok(())
}

fn validate_mode(mode: &Mode) -> Vec<String> {
    let mut issues = Vec::new();
    if mode.triggers_re.is_none() {
        issues.push("error: triggers contain no usable term".into());
    }
    // Migration lint: triggers used to be regex. `レビュー|review` is now one
    // literal term containing `|` that can never match — say so instead of
    // letting the mode silently stop attaching.
    for term in &mode.terms {
        if term.contains('|') {
            issues.push(format!(
                "warning: term {term:?} contains '|' — triggers are comma-separated literal terms, not regex"
            ));
        }
    }
    if mode.body.chars().count() > MAX_BODY_CHARS {
        issues.push(format!(
            "warning: body exceeds {MAX_BODY_CHARS} characters (agents truncate hook output)"
        ));
    }
    issues
}
