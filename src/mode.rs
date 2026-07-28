use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Mode {
    pub name: String,
    pub priority: i32,
    /// Original triggers text (for display / overlap hints).
    pub triggers: String,
    /// Compiled at load time. `None` when empty or invalid.
    pub triggers_re: Option<Regex>,
    pub body: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub chosen: Mode,
    /// Names of every mode that matched, winner first.
    pub matched_names: Vec<String>,
}

/// Parse a mode markdown file. Does not require a pre-compiled regex to succeed.
pub fn parse_mode(text: &str, path: &Path) -> Result<Mode> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let (front, body) = split_frontmatter(text);
    let mut name = stem;
    let mut priority = 0_i32;
    let mut triggers = String::new();

    for line in front.lines() {
        let line = line.trim().trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "name" => name = strip_quotes(value).to_string(),
            "priority" => {
                priority = strip_quotes(value)
                    .parse()
                    .with_context(|| format!("invalid priority in {}", path.display()))?;
            }
            "triggers" => triggers = parse_triggers(value)?,
            _ => {}
        }
    }

    let triggers_re = if triggers.is_empty() {
        None
    } else {
        Regex::new(&triggers).ok()
    };

    Ok(Mode {
        name,
        priority,
        triggers,
        triggers_re,
        body: body.trim().to_string(),
        path: path.to_path_buf(),
    })
}

fn split_frontmatter(text: &str) -> (&str, &str) {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return ("", text);
    };
    if let Some((front, body)) = rest.split_once("\n---\n") {
        return (front, body);
    }
    if let Some((front, body)) = rest.split_once("\r\n---\r\n") {
        return (front, body);
    }
    ("", text)
}

fn strip_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn parse_triggers(value: &str) -> Result<String> {
    let value = value.trim();
    if let Some(inner) = value.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let parts: Vec<&str> = inner
            .split(',')
            .map(|p| strip_quotes(p.trim()))
            .filter(|p| !p.is_empty())
            .collect();
        if parts.is_empty() {
            bail!("empty triggers list");
        }
        Ok(parts.join("|"))
    } else {
        Ok(strip_quotes(value).to_string())
    }
}

/// Collect `*.md` paths under modes dirs (sorted per directory).
pub fn list_mode_paths(dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .with_context(|| format!("Failed to read modes dir: {}", dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        entries.sort();
        paths.extend(entries);
    }
    Ok(paths)
}

/// Load modes from directories. Earlier directories win on duplicate names.
/// Unreadable or unparsable files are skipped (attach must stay resilient).
pub fn load_modes(dirs: &[PathBuf]) -> Result<Vec<Mode>> {
    let mut seen = HashSet::new();
    let mut modes = Vec::new();

    for path in list_mode_paths(dirs)? {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(mode) = parse_mode(&text, &path) else {
            continue;
        };
        if seen.insert(mode.name.clone()) {
            modes.push(mode);
        }
    }

    Ok(modes)
}

/// Select the best matching mode. Sort key: priority desc, name asc.
pub fn select(prompt: &str, modes: &[Mode]) -> Option<Selection> {
    let mut matched: Vec<&Mode> = modes
        .iter()
        .filter(|m| m.triggers_re.as_ref().is_some_and(|re| re.is_match(prompt)))
        .collect();

    if matched.is_empty() {
        return None;
    }

    matched.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.name.cmp(&b.name))
    });

    let matched_names: Vec<String> = matched.iter().map(|m| m.name.clone()).collect();
    Some(Selection {
        chosen: matched[0].clone(),
        matched_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mode(name: &str, priority: i32, triggers: &str) -> Mode {
        Mode {
            name: name.into(),
            priority,
            triggers: triggers.into(),
            triggers_re: Regex::new(triggers).ok(),
            body: format!("body of {name}"),
            path: PathBuf::from(format!("{name}.md")),
        }
    }

    #[test]
    fn strips_quotes_from_triggers() {
        let text = "---\nname: review\ntriggers: \"レビュー|review\"\n---\nbody\n";
        let m = parse_mode(text, Path::new("review.md")).unwrap();
        assert_eq!(m.triggers, "レビュー|review");
        assert!(m.triggers_re.unwrap().is_match("コードレビュー"));
    }

    #[test]
    fn priority_beats_name_order() {
        let modes = vec![
            mode("implement", 0, "実装|レビュー"),
            mode("review", 10, "レビュー"),
        ];
        let sel = select("この実装をレビューして", &modes).unwrap();
        assert_eq!(sel.chosen.name, "review");
        assert_eq!(sel.matched_names, vec!["review", "implement"]);
    }
}
