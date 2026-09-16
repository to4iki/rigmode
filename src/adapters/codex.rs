use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::adapters::{load_json, mode_context, write_json};
use crate::mode::Mode;
use crate::prompt::PromptMeta;

#[derive(Debug, Deserialize)]
struct CodexPayload {
    prompt: Option<String>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    cwd: Option<String>,
}

pub fn decode(stdin: &str) -> Result<PromptMeta> {
    let payload: CodexPayload =
        serde_json::from_str(stdin).context("stdin is not valid Codex hook JSON")?;
    Ok(PromptMeta {
        prompt: payload.prompt.unwrap_or_default(),
        session_id: payload.session_id,
        transcript_path: payload.transcript_path,
        cwd: payload.cwd,
    })
}

pub fn encode(modes: &[&Mode]) -> String {
    let context = mode_context(modes);
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
/// Codex runs `command` through `$SHELL -lc` and has no exec form, so our
/// arguments live at the tail of the command string. That tail is also the
/// idempotency key, so a reinstall from another binary path replaces the entry.
const ARGS: &str = "attach codex";

/// `$CODEX_HOME/hooks.json`, falling back to `~/.codex/hooks.json`.
pub fn hooks_path() -> std::path::PathBuf {
    let base = std::env::var_os("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".codex")
        });
    base.join("hooks.json")
}

/// The shell command Codex will run for `binary`.
pub fn command(binary: &std::path::Path) -> String {
    format!("{} {ARGS}", shell_quote(&binary.to_string_lossy()))
}

fn shell_quote(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+".contains(c));
    if plain {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

/// Inverse of `command`: the binary path when `cmd` is ours, else `None`.
fn binary_from_command(cmd: &str) -> Option<String> {
    let head = cmd.trim_end().strip_suffix(ARGS)?;
    if !head.ends_with(char::is_whitespace) {
        return None;
    }
    let quoted = head.trim_end();
    let unquoted = match quoted.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        Some(inner) => inner.replace(r"'\''", "'"),
        None => quoted.to_string(),
    };
    (!unquoted.is_empty()).then_some(unquoted)
}

fn is_our_entry(entry: &Value) -> bool {
    let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) else {
        return false;
    };
    hooks.iter().any(|hook| {
        hook.get("command")
            .and_then(|c| c.as_str())
            .is_some_and(|c| binary_from_command(c).is_some())
    })
}

fn our_entry(binary: &std::path::Path) -> Value {
    json!({
        "hooks": [{
            "type": "command",
            "command": command(binary),
            "timeout": TIMEOUT_SECONDS,
            // Codex spills additionalContext over ~2,500 tokens to a file and
            // hands the model a preview, which would drop the mode bodies.
            // 0 disables the spill; `check` already warns on oversized bodies.
            "additionalContextLimit": 0
        }]
    })
}

pub fn install_hook(hooks_path: &std::path::Path, binary: &std::path::Path) -> Result<()> {
    let mut root = load_json(hooks_path)?;
    let hooks = root
        .as_object_mut()
        .context("hooks.json root must be an object")?
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let hooks_obj = hooks.as_object_mut().context("hooks must be an object")?;
    let entries = hooks_obj.entry(EVENT).or_insert_with(|| json!([]));
    let list = entries
        .as_array_mut()
        .context("UserPromptSubmit must be an array")?;

    list.retain(|e| !is_our_entry(e));
    list.push(our_entry(binary));

    write_json(hooks_path, &root)
}

pub fn uninstall_hook(hooks_path: &std::path::Path) -> Result<bool> {
    if !hooks_path.exists() {
        return Ok(false);
    }
    let mut root = load_json(hooks_path)?;
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
        write_json(hooks_path, &root)?;
    }
    Ok(removed)
}

/// The registered binary path (unquoted), if our hook is present.
pub fn registered_command(hooks_path: &std::path::Path) -> Result<Option<String>> {
    if !hooks_path.exists() {
        return Ok(None);
    }
    let root = load_json(hooks_path)?;
    let Some(entries) = root
        .pointer("/hooks/UserPromptSubmit")
        .and_then(|e| e.as_array())
    else {
        return Ok(None);
    };
    for entry in entries {
        if let Some(cmd) = entry.pointer("/hooks/0/command").and_then(|c| c.as_str()) {
            if let Some(binary) = binary_from_command(cmd) {
                return Ok(Some(binary));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOREIGN: &str = r#"{
      "hooks": {
        "SessionStart": [{"hooks": [{"type": "command", "command": "bash '/x/state.sh' session", "timeout": 10}]}],
        "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "other-tool attach codex-not-us"}]}]
      }
    }"#;

    fn read(path: &std::path::Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn install_is_idempotent_by_command_tail() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join("hooks.json");
        std::fs::write(&hooks, FOREIGN).unwrap();

        let bin_a = dir.path().join("a/rigmode");
        let bin_b = dir.path().join("b/rigmode");
        install_hook(&hooks, &bin_a).unwrap();
        install_hook(&hooks, &bin_b).unwrap();

        let root = read(&hooks);
        let entries = root["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        let ours = &entries[1]["hooks"][0];
        assert_eq!(ours["type"], "command");
        assert_eq!(ours["command"], command(&bin_b));
        assert_eq!(ours["timeout"], TIMEOUT_SECONDS);
        assert_eq!(ours["additionalContextLimit"], 0);
        assert!(root["hooks"]["SessionStart"].is_array());
        assert_eq!(
            registered_command(&hooks).unwrap().as_deref(),
            Some(&*bin_b.to_string_lossy())
        );
    }

    #[test]
    fn uninstall_removes_only_ours() {
        let dir = tempfile::tempdir().unwrap();
        let hooks = dir.path().join("hooks.json");
        std::fs::write(&hooks, FOREIGN).unwrap();
        assert!(!uninstall_hook(&hooks).unwrap());

        install_hook(&hooks, &dir.path().join("rigmode")).unwrap();
        assert!(uninstall_hook(&hooks).unwrap());

        let root = read(&hooks);
        assert_eq!(
            root["hooks"]["UserPromptSubmit"].as_array().unwrap().len(),
            1
        );
        assert!(root["hooks"]["SessionStart"].is_array());
        assert_eq!(registered_command(&hooks).unwrap(), None);
    }

    #[test]
    fn command_round_trips_paths_needing_quotes() {
        for path in ["/usr/local/bin/rigmode", "/Users/me/My Apps/rig'mode"] {
            let cmd = command(std::path::Path::new(path));
            assert!(cmd.ends_with(" attach codex"), "{cmd}");
            assert_eq!(binary_from_command(&cmd).as_deref(), Some(path));
        }
        assert_eq!(
            command(std::path::Path::new("/Users/me/My Apps/rigmode")),
            "'/Users/me/My Apps/rigmode' attach codex"
        );
        assert_eq!(binary_from_command("rigmodeattach codex"), None);
        assert_eq!(binary_from_command("attach codex"), None);
        assert_eq!(binary_from_command("rigmode attach claude-code"), None);
    }

    #[test]
    fn decode_and_encode_match_codex_hook_shapes() {
        let meta = decode(
            r#"{"session_id":"s1","turn_id":"t1","cwd":"/w","transcript_path":null,
                "model":"m","permission_mode":"default","hook_event_name":"UserPromptSubmit",
                "prompt":"implement this"}"#,
        )
        .unwrap();
        assert_eq!(meta.prompt, "implement this");
        assert_eq!(meta.session_id.as_deref(), Some("s1"));
        assert_eq!(meta.cwd.as_deref(), Some("/w"));
        assert_eq!(meta.transcript_path, None);

        let mode = Mode {
            name: "implement".into(),
            terms: vec!["implement".into()],
            triggers_re: None,
            body: "## Gate\n\n- PR".into(),
            path: "implement.md".into(),
        };
        let out: Value = serde_json::from_str(&encode(&[&mode])).unwrap();
        assert_eq!(
            out["hookSpecificOutput"]["hookEventName"],
            "UserPromptSubmit"
        );
        let context = out["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .unwrap();
        assert!(context.starts_with("Work modes matching this request: implement."));
        assert!(context.contains("\n\n# implement\n\n## Gate"));
    }
}
