use std::path::Path;

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::config::GateConfig;
use crate::log::{self, AttachLogRecord};

pub const GATES_LOG: &str = "gates.jsonl";

/// One recorded intervention: the user pushed back on the agent's work.
#[derive(Debug, Serialize, Deserialize)]
pub struct GateRecord {
    pub ts: String,
    /// Modes attached by the session's last attach.
    pub modes: Vec<String>,
    /// The marker that matched.
    pub marker: String,
    /// First line of the prompt (truncated) for context.
    pub note: String,
    pub session_id: String,
}

const NOTE_MAX_CHARS: usize = 200;

/// Match the first line of a prompt against configured markers
/// (case-insensitive substring). Returns the marker and the (truncated) line.
fn detect_intervention(prompt: &str, gate: &GateConfig) -> Option<(String, String)> {
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

/// Modes attached by the session's last attach (newest hit wins).
fn resolve_modes_from_attach_log(path: &Path, session_id: &str) -> Option<Vec<String>> {
    log::list_jsonl(
        path,
        |r: &AttachLogRecord| r.session_id.as_deref() == Some(session_id),
        Some(1),
    )
    .pop()
    .map(|r| r.modes)
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
    let attach_path = data_dir.join(log::ATTACH_LOG);
    let Some(modes) = resolve_modes_from_attach_log(&attach_path, session_id) else {
        return;
    };

    let record = GateRecord {
        ts: Local::now().to_rfc3339(),
        modes,
        marker,
        note,
        session_id: session_id.to_string(),
    };
    log::append_jsonl(&data_dir.join(GATES_LOG), &record);
}

/// Newest first, optionally filtered by mode, limited.
pub fn list_gates(path: &Path, mode: Option<&str>, limit: Option<usize>) -> Vec<GateRecord> {
    log::list_jsonl(
        path,
        |r: &GateRecord| mode.is_none_or(|mode| r.modes.iter().any(|m| m == mode)),
        limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn records_intervention_with_last_attached_modes() {
        let dir = tempdir().unwrap();
        let data = dir.path();
        fs::write(
            data.join(log::ATTACH_LOG),
            r#"{"ts":"t1","session_id":"sid","cwd":null,"modes":["implement"]}
{"ts":"t2","session_id":"sid","cwd":null,"modes":["implement","review"]}
"#,
        )
        .unwrap();
        let gate = GateConfig {
            markers: vec!["違う".into(), "やり直し".into()],
        };

        maybe_record_from_prompt(data, "そこは違う、直して", Some("sid"), &gate);
        maybe_record_from_prompt(data, "続けて\n実は違う", Some("sid"), &gate); // marker not on first line
        maybe_record_from_prompt(data, "実装を続けて", Some("sid"), &gate); // no marker
        maybe_record_from_prompt(data, "違う", None, &gate); // no session
        maybe_record_from_prompt(data, "違う", Some("orphan"), &gate); // no prior attach
        maybe_record_from_prompt(data, "違う", Some("sid"), &GateConfig::default()); // markers unset

        let path = data.join(GATES_LOG);
        let gates = list_gates(&path, None, None);
        assert_eq!(gates.len(), 1);
        assert_eq!(gates[0].modes, vec!["implement", "review"]); // last attach wins
        assert_eq!(gates[0].marker, "違う");
        assert_eq!(gates[0].note, "そこは違う、直して");

        assert_eq!(list_gates(&path, Some("review"), None).len(), 1);
        assert!(list_gates(&path, Some("other"), None).is_empty());
    }
}
