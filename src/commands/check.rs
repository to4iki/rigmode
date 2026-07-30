use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use regex::Regex;

use crate::adapters::claude_code;
use crate::commands::attach;
use crate::config::Config;
use crate::mode::{self, Mode};

const MAX_BODY_CHARS: usize = 10_000;

pub fn execute(modes_dirs: Vec<PathBuf>, config: &Config) -> Result<()> {
    let dirs = attach::modes_dirs_for(modes_dirs, config)?;
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
    let mut modes = Vec::new();
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
        match mode::parse_mode(&text, path) {
            Ok(m) => {
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
                } else {
                    modes.push(m);
                }
            }
            Err(e) => {
                println!("  ! {}: error: {e:#}", path.display());
                errors += 1;
            }
        }
    }

    report_overlaps(&modes, &mut warnings);

    match claude_code::registered_command(&claude_code::settings_path())? {
        Some(cmd) => {
            let path = Path::new(&cmd);
            let status = if path.is_file() {
                "ok"
            } else {
                "missing binary"
            };
            println!("hook: registered -> {cmd} [{status}]");
            if !path.is_file() {
                errors += 1;
            }
        }
        None => {
            println!("hook: not registered");
            warnings += 1;
        }
    }

    if errors > 0 {
        bail!("{errors} error(s), {warnings} warning(s)");
    }
    println!("check passed ({warnings} warning(s))");
    Ok(())
}

fn validate_mode(mode: &Mode) -> Vec<String> {
    let mut issues = Vec::new();
    if mode.triggers.is_empty() {
        issues.push("error: empty triggers".into());
    } else if mode.triggers_re.is_none() {
        // Surface the compile error from regex.
        match Regex::new(&mode.triggers) {
            Ok(_) => {}
            Err(e) => {
                issues.push(format!("error: triggers do not compile as Rust regex: {e}"));
            }
        }
    }
    if mode.body.chars().count() > MAX_BODY_CHARS {
        issues.push(format!(
            "warning: body exceeds {MAX_BODY_CHARS} characters (Claude Code truncates hook output)"
        ));
    }
    issues
}

fn report_overlaps(modes: &[Mode], warnings: &mut i32) {
    for i in 0..modes.len() {
        for j in (i + 1)..modes.len() {
            let a = &modes[i];
            let b = &modes[j];
            let Some(re_a) = a.triggers_re.as_ref() else {
                continue;
            };
            let Some(re_b) = b.triggers_re.as_ref() else {
                continue;
            };
            let a_hits_b = re_a.is_match(&b.triggers) || re_a.is_match(&b.name);
            let b_hits_a = re_b.is_match(&a.triggers) || re_b.is_match(&a.name);
            if a_hits_b || b_hits_a {
                println!(
                    "  ! overlap: {} (priority {}) and {} (priority {}) may both match the same prompt",
                    a.name, a.priority, b.name, b.priority
                );
                *warnings += 1;
            }
        }
    }
}
