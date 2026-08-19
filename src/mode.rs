use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use regex::Regex;

#[derive(Debug)]
pub struct Mode {
    pub name: String,
    /// Parsed trigger terms, matched literally.
    pub terms: Vec<String>,
    /// Compiled at load time via [`build_pattern`]. `None` when no usable term.
    pub triggers_re: Option<Regex>,
    pub body: String,
    pub path: PathBuf,
}

/// Parse a mode markdown file. Never fails: bad frontmatter just yields a mode
/// with no usable triggers, which `check` reports.
pub fn parse_mode(text: &str, path: &Path) -> Mode {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let (front, body) = split_frontmatter(text);
    let mut name = stem;
    let mut terms = Vec::new();

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
            "triggers" => terms = parse_triggers(value),
            _ => {}
        }
    }

    let triggers_re = build_pattern(&terms);

    Mode {
        name,
        terms,
        triggers_re,
        body: body.trim().to_string(),
        path: path.to_path_buf(),
    }
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

/// Comma-separated terms, as a scalar or a `[a, b]` list. Quotes are stripped.
fn parse_triggers(value: &str) -> Vec<String> {
    let value = value.trim();
    let inner = value
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or_else(|| strip_quotes(value));
    inner
        .split(',')
        .map(|t| strip_quotes(t.trim()).trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Compile trigger / marker terms into one case-insensitive pattern.
///
/// Each term is matched literally, and an end that is an ASCII letter may not be
/// glued to another one, so `pr` stays out of `priority` (and `no` out of
/// `notification`). Only such ends are guarded: `実装して` has to keep matching
/// in `実装してPR作って`, and `\b` cannot draw that line, since regex counts kana
/// as word characters. The regex crate has no lookaround, so the guards consume
/// a neighbor char instead — fine for `is_match`, which is the only use.
pub(crate) fn build_pattern(terms: &[String]) -> Option<Regex> {
    let mut alts = Vec::new();
    for term in terms {
        let term = term.trim();
        if term.is_empty() {
            continue;
        }
        // A space inside a term tolerates being dropped: `pull request` / `pullrequest`.
        let mut word = term
            .split_whitespace()
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join(r"\s?");
        if term.starts_with(|c: char| c.is_ascii_alphabetic()) {
            word = format!("(?:^|[^A-Za-z]){word}");
        }
        if term.ends_with(|c: char| c.is_ascii_alphabetic()) {
            word = format!("{word}(?:[^A-Za-z]|$)");
        }
        alts.push(word);
    }
    if alts.is_empty() {
        return None;
    }
    Regex::new(&format!("(?i){}", alts.join("|"))).ok()
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
/// Unreadable files are skipped (attach must stay resilient).
pub fn load_modes(dirs: &[PathBuf]) -> Result<Vec<Mode>> {
    let mut seen = HashSet::new();
    let mut modes = Vec::new();

    for path in list_mode_paths(dirs)? {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mode = parse_mode(&text, &path);
        if seen.insert(mode.name.clone()) {
            modes.push(mode);
        }
    }

    Ok(modes)
}

/// Every mode whose triggers match, in load order (directory precedence,
/// filename order within). Modes are phases of one job — a request routinely
/// spans several (implement this, then open the PR), so picking a single
/// winner would drop the other phase's stop conditions. All of them attach
/// and their guardrails add up.
pub fn matching<'a>(prompt: &str, modes: &'a [Mode]) -> Vec<&'a Mode> {
    modes
        .iter()
        .filter(|m| m.triggers_re.as_ref().is_some_and(|re| re.is_match(prompt)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a mode through the real parse path, so tests survive internal
    /// refactors of term parsing / pattern compilation.
    fn mode(name: &str, triggers: &str) -> Mode {
        let text = format!("---\nname: {name}\ntriggers: {triggers}\n---\nbody of {name}\n");
        parse_mode(&text, Path::new(&format!("{name}.md")))
    }

    fn re(triggers: &str) -> Regex {
        mode("m", triggers).triggers_re.unwrap()
    }

    #[test]
    fn strips_quotes_from_triggers() {
        let m = mode("review", "\"レビュー, review\"");
        assert!(m.triggers_re.unwrap().is_match("コードレビュー"));
    }

    #[test]
    fn ascii_terms_do_not_match_inside_words() {
        let re = re("pr, pull request");
        assert!(re.is_match("PRを作って"));
        assert!(re.is_match("open a pr"));
        assert!(re.is_match("pr"));
        assert!(re.is_match("Pull Request お願い"));
        assert!(re.is_match("pullrequest"));
        assert!(!re.is_match("priority を上げて"));
        assert!(!re.is_match("apricot"));
    }

    #[test]
    fn kana_terms_match_inside_longer_text() {
        let re = re("実装して, レビュー");
        assert!(re.is_match("実装してPR作って"));
        assert!(re.is_match("コードレビューして"));
        assert!(!re.is_match("設計だけ考えて"));
    }

    #[test]
    fn terms_are_literal_not_regex() {
        let re = re("fix(bug)");
        assert!(re.is_match("please fix(bug) now"));
        assert!(!re.is_match("fixbug"));
    }

    #[test]
    fn no_usable_term_means_mode_never_attaches() {
        for triggers in ["", " ,  , ", "[]"] {
            let modes = vec![mode("m", triggers)];
            assert!(matching("なんでも", &modes).is_empty(), "{triggers:?}");
        }
    }

    #[test]
    fn all_matching_modes_attach_in_load_order() {
        let modes = vec![
            mode("implement", "実装して, 直して"),
            mode("pull-request", "pr, プルリク"),
            mode("review", "レビュー"),
        ];

        let matched = matching("実装してPR作って", &modes);
        let names: Vec<&str> = matched.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["implement", "pull-request"]);

        assert!(matching("設計だけ考えて", &modes).is_empty());
    }
}
