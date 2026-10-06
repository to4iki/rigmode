use serde_json::{Value, json};

use crate::adapters::{HookSpec, Registration, TIMEOUT_SECONDS, agent_home};

pub(crate) static SPEC: HookSpec = HookSpec {
    is_ours,
    registration,
    render,
};

pub fn settings_path() -> std::path::PathBuf {
    agent_home("CLAUDE_CONFIG_DIR", ".claude").join("settings.json")
}

/// Claude Code has an exec form, so idempotency keys on `args` rather than the
/// binary path.
fn is_ours(hook: &Value) -> bool {
    let Some(args) = hook.get("args").and_then(|a| a.as_array()) else {
        return false;
    };
    args.len() >= 2 && args[0].as_str() == Some("attach") && args[1].as_str() == Some("claude-code")
}

fn registration(hook: &Value) -> Registration {
    let command = hook
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    // Exec form: `command` is the executable, never a shell string.
    Registration {
        binary: (!command.is_empty()).then(|| command.clone()),
        command,
    }
}

fn render(binary: &std::path::Path) -> Value {
    json!({
        "type": "command",
        "command": binary.to_string_lossy(),
        "args": ["attach", "claude-code"],
        "timeout": TIMEOUT_SECONDS
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::Hooks;

    fn hooks(path: &std::path::Path) -> Hooks {
        Hooks::new(path.to_path_buf(), &SPEC)
    }

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
        hooks(&settings).install(&bin_a).unwrap();
        hooks(&settings).install(&bin_b).unwrap();

        let root: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let entries = root["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0]["hooks"][0]["command"].as_str().unwrap(),
            bin_b.to_string_lossy()
        );
        assert!(root["hooks"]["SessionStart"].is_array());
        assert_eq!(
            hooks(&settings).registered().unwrap().unwrap().binary,
            Some(bin_b.to_string_lossy().into_owned())
        );
    }

    /// A hook another tool put in the same entry survives, and is still found
    /// when it sits ahead of ours.
    #[test]
    fn install_and_uninstall_keep_sibling_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let settings = dir.path().join("settings.json");
        let bin = dir.path().join("rigmode");
        std::fs::write(
            &settings,
            format!(
                r#"{{"hooks":{{"UserPromptSubmit":[{{"hooks":[
                   {{"type":"command","command":"other-tool"}},
                   {{"type":"command","command":"{}","args":["attach","claude-code"]}}
                ]}}]}}}}"#,
                bin.to_string_lossy()
            ),
        )
        .unwrap();

        assert_eq!(
            hooks(&settings).registered().unwrap().unwrap().binary,
            Some(bin.to_string_lossy().into_owned())
        );

        assert!(hooks(&settings).uninstall().unwrap());
        let root: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let entries = root["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["hooks"].as_array().unwrap().len(), 1);
        assert_eq!(entries[0]["hooks"][0]["command"], "other-tool");
    }
}
