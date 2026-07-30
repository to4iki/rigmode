use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mode::Mode;

#[derive(Debug, Clone, Default)]
pub struct PromptMeta {
    pub prompt: String,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudePayload {
    prompt: Option<String>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    cwd: Option<String>,
}

pub fn decode(stdin: &str) -> Result<PromptMeta> {
    let payload: ClaudePayload =
        serde_json::from_str(stdin).context("stdin is not valid Claude Code hook JSON")?;
    Ok(PromptMeta {
        prompt: payload.prompt.unwrap_or_default(),
        session_id: payload.session_id,
        transcript_path: payload.transcript_path,
        cwd: payload.cwd,
    })
}

pub fn encode(mode: &Mode) -> String {
    let context = format!(
        "The work mode matching this request is {}. Its contents follow. \
         Hook output does not reach subagents; delegating copies this body into subagent instructions.\n\n{}",
        mode.name, mode.body
    );
    json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": context
        }
    })
    .to_string()
}

const EVENT: &str = "UserPromptSubmit";
const TIMEOUT_SECONDS: u64 = 5;

pub fn settings_path() -> std::path::PathBuf {
    let base = std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".claude")
        });
    base.join("settings.json")
}

fn is_our_entry(entry: &Value) -> bool {
    let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) else {
        return false;
    };
    hooks.iter().any(|hook| {
        let Some(args) = hook.get("args").and_then(|a| a.as_array()) else {
            return false;
        };
        args.len() >= 2
            && args[0].as_str() == Some("attach")
            && args[1].as_str() == Some("claude-code")
    })
}

fn our_entry(binary: &std::path::Path) -> Value {
    json!({
        "hooks": [{
            "type": "command",
            "command": binary.to_string_lossy(),
            "args": ["attach", "claude-code"],
            "timeout": TIMEOUT_SECONDS
        }]
    })
}

pub fn install_hook(settings_path: &std::path::Path, binary: &std::path::Path) -> Result<()> {
    let mut root = load_settings(settings_path)?;
    let hooks = root
        .as_object_mut()
        .context("settings.json root must be an object")?
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let hooks_obj = hooks.as_object_mut().context("hooks must be an object")?;
    let entries = hooks_obj.entry(EVENT).or_insert_with(|| json!([]));
    let list = entries
        .as_array_mut()
        .context("UserPromptSubmit must be an array")?;

    list.retain(|e| !is_our_entry(e));
    list.push(our_entry(binary));

    write_settings(settings_path, &root)
}

pub fn uninstall_hook(settings_path: &std::path::Path) -> Result<bool> {
    if !settings_path.exists() {
        return Ok(false);
    }
    let mut root = load_settings(settings_path)?;
    let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(false);
    };
    let Some(entries) = hooks.get_mut(EVENT).and_then(|e| e.as_array_mut()) else {
        return Ok(false);
    };
    let before = entries.len();
    entries.retain(|e| !is_our_entry(e));
    let removed = entries.len() != before;

    if entries.is_empty() {
        hooks.remove(EVENT);
    }
    if hooks.is_empty() {
        if let Some(obj) = root.as_object_mut() {
            obj.remove("hooks");
        }
    }

    if removed {
        write_settings(settings_path, &root)?;
    }
    Ok(removed)
}

pub fn registered_command(settings_path: &std::path::Path) -> Result<Option<String>> {
    if !settings_path.exists() {
        return Ok(None);
    }
    let root = load_settings(settings_path)?;
    let Some(entries) = root
        .pointer("/hooks/UserPromptSubmit")
        .and_then(|e| e.as_array())
    else {
        return Ok(None);
    };
    for entry in entries {
        if is_our_entry(entry) {
            if let Some(cmd) = entry.pointer("/hooks/0/command").and_then(|c| c.as_str()) {
                return Ok(Some(cmd.to_string()));
            }
        }
    }
    Ok(None)
}

fn load_settings(path: &std::path::Path) -> Result<Value> {
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

fn write_settings(path: &std::path::Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(value)?;
    std::fs::write(path, format!("{body}\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_is_idempotent_by_args() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        std::fs::write(
            &settings,
            r#"{"hooks":{"SessionStart":[{"hooks":[{"command":"keep-me"}]}]}}"#,
        )
        .unwrap();

        let bin_a = dir.path().join("a/rigmode");
        let bin_b = dir.path().join("b/rigmode");
        install_hook(&settings, &bin_a).unwrap();
        install_hook(&settings, &bin_b).unwrap();

        let root: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let entries = root["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0]["hooks"][0]["command"].as_str().unwrap(),
            bin_b.to_string_lossy()
        );
        assert!(root["hooks"]["SessionStart"].is_array());
    }
}
