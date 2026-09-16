pub mod claude_code;
pub mod codex;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::mode::Mode;

/// The agent-agnostic context text every adapter injects for the matched modes.
pub fn mode_context(modes: &[&Mode]) -> String {
    use std::fmt::Write;

    let names: Vec<&str> = modes.iter().map(|m| m.name.as_str()).collect();
    let mut context = format!(
        "Work modes matching this request: {}. All of them apply, so satisfy every \
         stop condition and respect every gate below. Hook output does not reach \
         subagents, so delegating requires copying these bodies into the subagent \
         instructions.",
        names.join(", ")
    );
    // Name each body, so a section heading is never read as the other mode's.
    for m in modes {
        let _ = write!(context, "\n\n# {}\n\n{}", m.name, m.body);
    }
    context
}

/// Missing or blank file is an empty object; invalid JSON is an error so we
/// never overwrite a file we could not read.
pub(crate) fn load_json(path: &std::path::Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    match serde_json::from_str(&text) {
        Ok(v) => Ok(v),
        Err(e) => bail!(
            "{} is not valid JSON, refusing to overwrite it: {e}",
            path.display()
        ),
    }
}

pub(crate) fn write_json(path: &std::path::Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(value)?;
    std::fs::write(path, format!("{body}\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
