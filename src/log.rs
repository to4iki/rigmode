use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::adapters::claude_code::PromptMeta;
use crate::mode::Mode;

#[derive(Debug, Serialize)]
struct AttachRecord<'a> {
    ts: String,
    agent: &'a str,
    session_id: Option<&'a str>,
    transcript_path: Option<&'a str>,
    cwd: Option<&'a str>,
    /// Every attached mode, in injection order.
    modes: Vec<&'a str>,
}

/// One recorded attach, read back from attach.jsonl.
#[derive(Debug, Clone, Deserialize)]
pub struct AttachLogRecord {
    #[serde(default)]
    pub ts: String,
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub modes: Vec<String>,
}

/// Best-effort append. Failures are swallowed so attach stays exit 0.
pub fn append_attach(path: &Path, agent: &str, meta: &PromptMeta, modes: &[&Mode]) {
    let record = AttachRecord {
        ts: Local::now().to_rfc3339(),
        agent,
        session_id: meta.session_id.as_deref(),
        transcript_path: meta.transcript_path.as_deref(),
        cwd: meta.cwd.as_deref(),
        modes: modes.iter().map(|m| m.name.as_str()).collect(),
    };

    let Ok(line) = serde_json::to_string(&record) else {
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

/// Parse a JSONL log newest-first (file order is append-only), keeping records
/// that pass `keep`, stopping once `limit` are found. Unparseable lines are
/// skipped; missing file → empty.
pub(crate) fn list_jsonl<T: serde::de::DeserializeOwned>(
    path: &Path,
    keep: impl Fn(&T) -> bool,
    limit: Option<usize>,
) -> Vec<T> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .filter(|r| keep(r))
        .take(limit.unwrap_or(usize::MAX))
        .collect()
}

/// Newest first, optionally filtered by attached mode, limited.
/// Records without `modes` (pre-rename schema) are ignored.
pub fn list_attaches(
    path: &Path,
    mode: Option<&str>,
    limit: Option<usize>,
) -> Vec<AttachLogRecord> {
    list_jsonl(
        path,
        |r: &AttachLogRecord| match mode {
            Some(mode) => r.modes.iter().any(|m| m == mode),
            None => !r.modes.is_empty(),
        },
        limit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn list_newest_first_with_filter_and_limit() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("attach.jsonl");
        fs::write(
            &path,
            r#"{"ts":"t0","agent":"claude-code","session_id":"s0","cwd":"/z","chosen":"old-schema"}
{"ts":"t1","agent":"claude-code","session_id":"s1","cwd":"/a","modes":["implement"]}
{"ts":"t2","agent":"claude-code","session_id":"s2","cwd":"/b","modes":["implement","review"]}
not json
"#,
        )
        .unwrap();

        let all = list_attaches(&path, None, None);
        assert_eq!(all.len(), 2); // pre-rename t0 is ignored
        assert_eq!(all[0].ts, "t2"); // newest first
        assert_eq!(all[0].modes, vec!["implement", "review"]);

        assert_eq!(list_attaches(&path, Some("review"), None).len(), 1);
        assert_eq!(list_attaches(&path, Some("implement"), None).len(), 2);
        assert_eq!(list_attaches(&path, None, Some(1)).len(), 1);
        assert!(list_attaches(&path, Some("other"), None).is_empty());
        assert!(list_attaches(&dir.path().join("missing.jsonl"), None, None).is_empty());
    }
}
