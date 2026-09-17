pub mod claude_code;
pub mod codex;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::cli::Agent;
use crate::mode::Mode;
use crate::prompt::PromptMeta;

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

/// Both agents send the same `UserPromptSubmit` payload; fields only one of
/// them sends (Codex's `turn_id`, `model`, ...) are ignored.
#[derive(Debug, Deserialize)]
struct HookPayload {
    prompt: Option<String>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    cwd: Option<String>,
}

pub fn decode(stdin: &str, agent: Agent) -> Result<PromptMeta> {
    let payload: HookPayload = serde_json::from_str(stdin)
        .with_context(|| format!("stdin is not valid {} hook JSON", agent.as_str()))?;
    Ok(PromptMeta {
        prompt: payload.prompt.unwrap_or_default(),
        session_id: payload.session_id,
        transcript_path: payload.transcript_path,
        cwd: payload.cwd,
    })
}

/// Both agents read the modes back out of `hookSpecificOutput.additionalContext`.
pub fn encode(modes: &[&Mode]) -> String {
    json!({
        "hookSpecificOutput": {
            "hookEventName": EVENT,
            "additionalContext": mode_context(modes)
        }
    })
    .to_string()
}

const EVENT: &str = "UserPromptSubmit";
/// Hang protection: attach only reads local files. Codex would otherwise wait
/// its 600 s default, and on timeout it skips the modes without erasing the
/// prompt. The value is part of the Codex trust hash, so changing it means
/// re-trusting the hook.
pub(crate) const TIMEOUT_SECONDS: u64 = 5;

/// What an agent's hook file currently records for rigmode.
pub struct Registration {
    /// The entry as written, for display.
    pub command: String,
    /// The executable it runs, when the entry is one rigmode itself could have
    /// written. `None` for a hand-edited wrapper whose head is not a plain
    /// path, so `check` never claims to have verified something it cannot.
    pub binary: Option<String>,
}

/// The agent-specific half of hook registration: recognising rigmode's hook
/// object inside the file, reading it back, and rendering a fresh one.
pub(crate) struct HookSpec {
    pub is_ours: fn(&Value) -> bool,
    pub registration: fn(&Value) -> Registration,
    pub render: fn(&std::path::Path) -> Value,
}

/// One agent's hook file plus the spec for reading and writing our entry in it.
pub struct Hooks {
    pub path: std::path::PathBuf,
    spec: &'static HookSpec,
}

pub fn hooks(agent: Agent) -> Hooks {
    match agent {
        Agent::ClaudeCode => Hooks::new(claude_code::settings_path(), &claude_code::SPEC),
        Agent::Codex => Hooks::new(codex::hooks_path(), &codex::SPEC),
    }
}

impl Hooks {
    pub(crate) fn new(path: std::path::PathBuf, spec: &'static HookSpec) -> Self {
        Hooks { path, spec }
    }

    pub fn install(&self, binary: &std::path::Path) -> Result<()> {
        let mut root = load_json(&self.path)?;
        let hooks = root
            .as_object_mut()
            .with_context(|| format!("{} root must be an object", self.path.display()))?
            .entry("hooks")
            .or_insert_with(|| json!({}));
        let hooks_obj = hooks.as_object_mut().context("hooks must be an object")?;
        let entries = hooks_obj.entry(EVENT).or_insert_with(|| json!([]));
        let list = entries
            .as_array_mut()
            .context("UserPromptSubmit must be an array")?;

        self.drop_ours(list);
        list.push(json!({ "hooks": [(self.spec.render)(binary)] }));

        write_json(&self.path, &root)
    }

    pub fn uninstall(&self) -> Result<bool> {
        let mut root = load_json(&self.path)?;
        let Some(hooks) = root.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
            return Ok(false);
        };
        let Some(entries) = hooks.get_mut(EVENT).and_then(|e| e.as_array_mut()) else {
            return Ok(false);
        };
        let removed = self.drop_ours(entries);

        if entries.is_empty() {
            hooks.remove(EVENT);
        }
        if hooks.is_empty() {
            if let Some(obj) = root.as_object_mut() {
                obj.remove("hooks");
            }
        }

        if removed {
            write_json(&self.path, &root)?;
        }
        Ok(removed)
    }

    /// Our registration in this file, if any.
    pub fn registered(&self) -> Result<Option<Registration>> {
        let root = load_json(&self.path)?;
        let Some(entries) = root
            .get("hooks")
            .and_then(|h| h.get(EVENT))
            .and_then(|e| e.as_array())
        else {
            return Ok(None);
        };
        // Scan every hook of every entry, matching `drop_ours`: a predicate
        // that disagrees with it would report "not registered" for a hook
        // install/uninstall do act on.
        Ok(entries
            .iter()
            .filter_map(|entry| entry.get("hooks").and_then(|h| h.as_array()))
            .flatten()
            .find(|hook| (self.spec.is_ours)(hook))
            .map(self.spec.registration))
    }

    /// Removes our hook objects, leaving hooks another tool put in the same
    /// entry alone. An entry we emptied goes with them.
    fn drop_ours(&self, entries: &mut Vec<Value>) -> bool {
        let mut removed = false;
        entries.retain_mut(|entry| {
            let Some(hooks) = entry.get_mut("hooks").and_then(|h| h.as_array_mut()) else {
                return true;
            };
            let before = hooks.len();
            hooks.retain(|hook| !(self.spec.is_ours)(hook));
            removed |= hooks.len() != before;
            before == 0 || !hooks.is_empty()
        });
        removed
    }
}

/// `$VAR`, falling back to `~/<dir>`. An empty `$VAR` is treated as unset, so
/// a wrapper exporting it blank does not turn the path relative to `$PWD`.
pub(crate) fn agent_home(var: &str, dir: &str) -> std::path::PathBuf {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(dir)
        })
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
        Err(e) => bail!("{} is not valid JSON: {e}", path.display()),
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
