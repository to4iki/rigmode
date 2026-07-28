use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use chrono::Local;
use serde::Serialize;

use crate::adapters::claude_code::PromptMeta;
use crate::mode::Selection;

#[derive(Debug, Serialize)]
struct AttachRecord<'a> {
    ts: String,
    agent: &'a str,
    session_id: Option<&'a str>,
    transcript_path: Option<&'a str>,
    cwd: Option<&'a str>,
    chosen: &'a str,
    matched: Vec<&'a str>,
}

/// Best-effort append. Failures are swallowed so attach stays exit 0.
pub fn append_attach(
    path: &Path,
    agent: &str,
    meta: &PromptMeta,
    selection: &Selection,
) {
    let record = AttachRecord {
        ts: Local::now().to_rfc3339(),
        agent,
        session_id: meta.session_id.as_deref(),
        transcript_path: meta.transcript_path.as_deref(),
        cwd: meta.cwd.as_deref(),
        chosen: &selection.chosen.name,
        matched: selection.matched_names.iter().map(|s| s.as_str()).collect(),
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
