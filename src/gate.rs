use std::fs::{self, OpenOptions};
use std::io::{BufRead, Write};
use std::path::Path;

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::GateConfig;

/// One recorded intervention: the user pushed back on the agent's work.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateRecord {
    pub ts: String,
    pub mode: String,
    /// The marker that matched.
    pub marker: String,
    /// First line of the prompt (truncated) for context.
    pub note: String,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
struct AttachLine {
    session_id: Option<String>,
    chosen: Option<String>,
}

const NOTE_MAX_CHARS: usize = 200;

/// Match the first line of a prompt against configured markers
/// (case-insensitive substring). Returns the marker and the (truncated) line.
pub fn detect_intervention(prompt: &str, gate: &GateConfig) -> Option<(String, String)> {
    let first = prompt.lines().next()?.trim();
    if first.is_empty() {
        return None;
    }
    let lower = first.to_lowercase();
    let marker = gate
        .markers
        .iter()
        .filter(|m| !m.trim().is_empty())
        .find(|m| lower.contains(&m.to_lowercase()))
        .cloned()?;
    Some((marker, truncate_chars(first, NOTE_MAX_CHARS)))
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

/// Last `chosen` mode for `session_id` in attach.jsonl (file order; last wins).
pub fn resolve_mode_from_attach_log(path: &Path, session_id: &str) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut found = None;
    for line in reader.lines() {
        let Ok(line) = line else {
            continue;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(rec) = serde_json::from_str::<AttachLine>(line) else {
            continue;
        };
        if rec.session_id.as_deref() == Some(session_id) {
            if let Some(chosen) = rec.chosen {
                found = Some(chosen);
            }
        }
    }
    found
}

/// If the prompt's first line contains a configured marker and a prior attach
/// exists for the session, append a record. Best-effort: failures are swallowed
/// so attach stays exit 0.
pub fn maybe_record_from_prompt(
    data_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
    gate: &GateConfig,
) {
    let Some(session_id) = session_id else {
        return;
    };
    let Some((marker, note)) = detect_intervention(prompt, gate) else {
        return;
    };
    let attach_path = data_dir.join("attach.jsonl");
    let Some(mode) = resolve_mode_from_attach_log(&attach_path, session_id) else {
        return;
    };

    let record = GateRecord {
        ts: Local::now().to_rfc3339(),
        mode,
        marker,
        note,
        session_id: session_id.to_string(),
    };
    append_record(&data_dir.join("gates.jsonl"), &record);
}

fn append_record(path: &Path, record: &GateRecord) {
    let Ok(line) = serde_json::to_string(record) else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

/// Newest first, optionally filtered by mode, limited. Missing file → empty.
pub fn list_gates(path: &Path, mode: Option<&str>, limit: Option<usize>) -> Vec<GateRecord> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut records: Vec<GateRecord> = text
        .lines()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .collect();
    if let Some(mode) = mode {
        records.retain(|r| r.mode == mode);
    }
    records.reverse();
    if let Some(limit) = limit {
        records.truncate(limit);
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn gate_config() -> GateConfig {
        GateConfig {
            markers: vec!["違う".into(), "やり直し".into()],
        }
    }

    #[test]
    fn detects_marker_on_first_line_only() {
        let gate = gate_config();

        let (marker, note) = detect_intervention("いや、違う方向で", &gate).unwrap();
        assert_eq!(marker, "違う");
        assert_eq!(note, "いや、違う方向で");

        assert!(detect_intervention("続けて\n実は違う", &gate).is_none());
        assert!(detect_intervention("実装して", &gate).is_none());
        assert!(detect_intervention("違う", &GateConfig::default()).is_none());
    }

    #[test]
    fn records_when_attach_exists_and_skips_otherwise() {
        let dir = tempdir().unwrap();
        let data = dir.path();
        fs::write(
            data.join("attach.jsonl"),
            r#"{"ts":"t","session_id":"sid","chosen":"review"}
"#,
        )
        .unwrap();
        let gate = gate_config();

        maybe_record_from_prompt(data, "そこは違う、直して", Some("sid"), &gate);
        maybe_record_from_prompt(data, "実装を続けて", Some("sid"), &gate); // no marker
        maybe_record_from_prompt(data, "違う", None, &gate); // no session
        maybe_record_from_prompt(data, "違う", Some("orphan"), &gate); // no attach

        let gates = list_gates(&data.join("gates.jsonl"), None, None);
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].mode, "review");
        assert_eq!(gates[0].marker, "違う");
        assert_eq!(gates[0].note, "そこは違う、直して");
    }

    #[test]
    fn list_newest_first_with_filter_and_limit() {
        let dir = tempdir().unwrap();
        let data = dir.path();
        fs::write(
            data.join("attach.jsonl"),
            r#"{"ts":"t","session_id":"s","chosen":"review"}
"#,
        )
        .unwrap();
        let gate = gate_config();
        maybe_record_from_prompt(data, "違う A", Some("s"), &gate);
        maybe_record_from_prompt(data, "やり直し B", Some("s"), &gate);

        let path = data.join("gates.jsonl");
        let all = list_gates(&path, None, None);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].note, "やり直し B"); // newest first

        assert_eq!(list_gates(&path, Some("review"), Some(1)).len(), 1);
        assert!(list_gates(&path, Some("other"), None).is_empty());
    }

    #[test]
    fn resolve_mode_takes_last_for_session() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("attach.jsonl");
        fs::write(
            &path,
            r#"{"ts":"t1","session_id":"s1","chosen":"implement"}
{"ts":"t2","session_id":"s1","chosen":"review"}
"#,
        )
        .unwrap();
        assert_eq!(
            resolve_mode_from_attach_log(&path, "s1").as_deref(),
            Some("review")
        );
        assert!(resolve_mode_from_attach_log(&path, "missing").is_none());
    }
}
