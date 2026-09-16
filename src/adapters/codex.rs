use serde_json::{Value, json};

use crate::adapters::{HookSpec, Registration, TIMEOUT_SECONDS, agent_home};

pub(crate) static SPEC: HookSpec = HookSpec {
    is_ours,
    registration,
    render,
};

/// Codex runs `command` through `$SHELL -lc` and has no exec form, so our
/// arguments live in the command string. They are also the idempotency key, so
/// a reinstall from another binary path replaces the entry.
const ARGS: &str = "attach codex";

/// `$CODEX_HOME/hooks.json`, falling back to `~/.codex/hooks.json`.
pub fn hooks_path() -> std::path::PathBuf {
    agent_home("CODEX_HOME", ".codex").join("hooks.json")
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

/// The command text before our `attach codex` marker, or `None` when the
/// command is not ours. Matching on a token boundary rather than the end of
/// the string keeps a hand-added flag (`... attach codex --modes-dir /m`) ours,
/// so a reinstall replaces it instead of appending a second hook.
fn our_head(cmd: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(offset) = cmd[from..].find(ARGS) {
        let start = from + offset;
        let end = start + ARGS.len();
        if cmd[..start].ends_with(char::is_whitespace)
            && cmd[end..].chars().next().is_none_or(char::is_whitespace)
        {
            return Some(cmd[..start].trim());
        }
        from = end;
    }
    None
}

/// Inverse of `command`: the binary path when `head` is one `command` itself
/// could have produced. A wrapped, prefixed or `$PATH`-resolved head
/// (`env FOO=1 rigmode`, `bash '/x/wrap.sh'`, `rigmode`) is still ours but is
/// not a path anything can stat, so it stays `None` rather than being reported
/// as a missing binary.
fn binary_from_head(head: &str) -> Option<String> {
    let unquoted = match head.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        Some(inner) => inner.replace(r"'\''", "'"),
        None => head.to_string(),
    };
    (std::path::Path::new(&unquoted).is_absolute() && shell_quote(&unquoted) == head)
        .then_some(unquoted)
}

fn is_ours(hook: &Value) -> bool {
    hook.get("command")
        .and_then(|c| c.as_str())
        .and_then(our_head)
        .is_some()
}

fn registration(hook: &Value) -> Registration {
    let command = hook
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .to_string();
    let binary = our_head(&command).and_then(binary_from_head);
    Registration { command, binary }
}

fn render(binary: &std::path::Path) -> Value {
    json!({
        "type": "command",
        "command": command(binary),
        "timeout": TIMEOUT_SECONDS,
        // Codex spills additionalContext over ~2,500 tokens to a file and
        // hands the model a preview, which would drop the mode bodies.
        // 0 disables the spill; `check` already warns on oversized bodies.
        "additionalContextLimit": 0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{Hooks, decode, encode};
    use crate::cli::Agent;
    use crate::mode::Mode;

    const FOREIGN: &str = r#"{
      "hooks": {
        "SessionStart": [{"hooks": [{"type": "command", "command": "bash '/x/state.sh' session", "timeout": 10}]}],
        "UserPromptSubmit": [{"hooks": [{"type": "command", "command": "other-tool attach codex-not-us"}]}]
      }
    }"#;

    fn hooks(path: &std::path::Path) -> Hooks {
        Hooks::new(path.to_path_buf(), &SPEC)
    }

    fn read(path: &std::path::Path) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn binary_from_command(cmd: &str) -> Option<String> {
        our_head(cmd).and_then(binary_from_head)
    }

    #[test]
    fn install_is_idempotent_by_command_tail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        std::fs::write(&path, FOREIGN).unwrap();

        let bin_a = dir.path().join("a/rigmode");
        let bin_b = dir.path().join("b/rigmode");
        hooks(&path).install(&bin_a).unwrap();
        hooks(&path).install(&bin_b).unwrap();

        let root = read(&path);
        let entries = root["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        let ours = &entries[1]["hooks"][0];
        assert_eq!(ours["type"], "command");
        assert_eq!(ours["command"], command(&bin_b));
        assert_eq!(ours["timeout"], TIMEOUT_SECONDS);
        assert_eq!(ours["additionalContextLimit"], 0);
        assert!(root["hooks"]["SessionStart"].is_array());
        assert_eq!(
            hooks(&path)
                .registered()
                .unwrap()
                .unwrap()
                .binary
                .as_deref(),
            Some(&*bin_b.to_string_lossy())
        );
    }

    /// A hand-added flag must not make our own hook invisible to us, or
    /// install would append a second hook that injects every body twice.
    #[test]
    fn install_replaces_a_hand_edited_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        let bin = dir.path().join("rigmode");
        std::fs::write(
            &path,
            format!(
                r#"{{"hooks":{{"UserPromptSubmit":[{{"hooks":[
                   {{"type":"command","command":"{} attach codex --modes-dir /m"}}
                ]}}]}}}}"#,
                bin.to_string_lossy()
            ),
        )
        .unwrap();

        assert_eq!(
            hooks(&path)
                .registered()
                .unwrap()
                .unwrap()
                .binary
                .as_deref(),
            Some(&*bin.to_string_lossy())
        );
        hooks(&path).install(&bin).unwrap();
        let root = read(&path);
        assert_eq!(
            root["hooks"]["UserPromptSubmit"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    fn uninstall_removes_only_ours() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        std::fs::write(&path, FOREIGN).unwrap();
        assert!(!hooks(&path).uninstall().unwrap());

        hooks(&path).install(&dir.path().join("rigmode")).unwrap();
        assert!(hooks(&path).uninstall().unwrap());

        let root = read(&path);
        assert_eq!(
            root["hooks"]["UserPromptSubmit"].as_array().unwrap().len(),
            1
        );
        assert!(root["hooks"]["SessionStart"].is_array());
        assert!(hooks(&path).registered().unwrap().is_none());
    }

    /// Codex groups handlers under one entry, so removing ours must not take a
    /// sibling with it.
    #[test]
    fn uninstall_keeps_a_sibling_hook_in_the_same_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hooks.json");
        let bin = dir.path().join("rigmode");
        std::fs::write(
            &path,
            format!(
                r#"{{"hooks":{{"UserPromptSubmit":[{{"hooks":[
                   {{"type":"command","command":"other-tool run"}},
                   {{"type":"command","command":"{}"}}
                ]}}]}}}}"#,
                command(&bin)
            ),
        )
        .unwrap();

        // Ours sits at index 1, so it must still be reported as registered.
        assert_eq!(
            hooks(&path)
                .registered()
                .unwrap()
                .unwrap()
                .binary
                .as_deref(),
            Some(&*bin.to_string_lossy())
        );

        assert!(hooks(&path).uninstall().unwrap());
        let root = read(&path);
        let entries = root["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["hooks"].as_array().unwrap().len(), 1);
        assert_eq!(entries[0]["hooks"][0]["command"], "other-tool run");
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

    /// A wrapped command is ours (so install replaces it) but its head is not
    /// a path, so `check` must not report it as a missing binary.
    #[test]
    fn wrapped_commands_are_ours_without_a_checkable_binary() {
        for cmd in [
            "bash '/x/wrap.sh' attach codex",
            "env RUST_LOG=off /usr/bin/rigmode attach codex",
            "\"/My Apps/rigmode\" attach codex",
            "rigmode attach codex",
        ] {
            assert!(our_head(cmd).is_some(), "{cmd}");
            assert_eq!(binary_from_command(cmd), None, "{cmd}");
        }
    }

    #[test]
    fn decode_and_encode_match_codex_hook_shapes() {
        let meta = decode(
            r#"{"session_id":"s1","turn_id":"t1","cwd":"/w","transcript_path":null,
                "model":"m","permission_mode":"default","hook_event_name":"UserPromptSubmit",
                "prompt":"implement this"}"#,
            Agent::Codex,
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
