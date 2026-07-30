#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::adapters::claude_code::{self, PromptMeta};
use crate::cli::Agent;
use crate::config::{self, Config};
use crate::gate;
use crate::log;
use crate::mode::{self, Selection};

/// Always returns ExitCode::SUCCESS. Errors become empty stdout.
pub fn execute(agent: Agent, modes_dirs: Vec<PathBuf>) -> ExitCode {
    let output = run(agent, modes_dirs).unwrap_or_default();
    print!("{output}");
    ExitCode::SUCCESS
}

fn run(agent: Agent, modes_dirs: Vec<PathBuf>) -> anyhow::Result<String> {
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin)?;

    let dirs = resolve_modes_dirs(modes_dirs)?;
    let modes = mode::load_modes(&dirs)?;

    match agent {
        Agent::ClaudeCode => {
            let meta = claude_code::decode(&stdin)?;
            maybe_record_gate(&meta);
            let Some(selection) = mode::select(&meta.prompt, &modes) else {
                return Ok(String::new());
            };
            append_log(agent, &meta, &selection);
            Ok(claude_code::encode(&selection.chosen))
        }
    }
}

fn maybe_record_gate(meta: &PromptMeta) {
    let Ok(data_dir) = config::default_data_dir() else {
        return;
    };
    // Best-effort: unreadable config just disables gate markers.
    let gate_config = config::default_config_path()
        .and_then(|p| config::load_config(&p))
        .map(|c| c.gate)
        .unwrap_or_default();
    gate::maybe_record_from_prompt(
        &data_dir,
        &meta.prompt,
        meta.session_id.as_deref(),
        &gate_config,
    );
}

fn resolve_modes_dirs(override_dirs: Vec<PathBuf>) -> anyhow::Result<Vec<PathBuf>> {
    if !override_dirs.is_empty() {
        return Ok(override_dirs);
    }
    let config_path = config::default_config_path()?;
    let config = config::load_config(&config_path)?;
    config.resolved_modes_dirs()
}

fn append_log(agent: Agent, meta: &PromptMeta, selection: &Selection) {
    let Ok(data_dir) = config::default_data_dir() else {
        return;
    };
    log::append_attach(
        &data_dir.join("attach.jsonl"),
        agent.as_str(),
        meta,
        selection,
    );
}

/// Shared helper for check/explain to resolve modes dirs.
pub fn modes_dirs_for(
    override_dirs: Vec<PathBuf>,
    config: &Config,
) -> anyhow::Result<Vec<PathBuf>> {
    if !override_dirs.is_empty() {
        Ok(override_dirs)
    } else {
        config.resolved_modes_dirs()
    }
}
