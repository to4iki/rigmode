#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::adapters::claude_code;
use crate::cli::Agent;
use crate::config;
use crate::gate;
use crate::log;
use crate::mode;

/// Always returns ExitCode::SUCCESS. Errors become empty stdout.
pub fn execute(agent: Agent, modes_dirs: Vec<PathBuf>) -> ExitCode {
    let output = run(agent, modes_dirs).unwrap_or_default();
    print!("{output}");
    ExitCode::SUCCESS
}

fn run(agent: Agent, modes_dirs: Vec<PathBuf>) -> anyhow::Result<String> {
    let mut stdin = String::new();
    io::stdin().read_to_string(&mut stdin)?;

    match agent {
        Agent::ClaudeCode => {
            let meta = claude_code::decode(&stdin)?;
            // Best-effort: an unreadable config still attaches with defaults.
            let config = config::default_config_path()
                .and_then(|p| config::load_config(&p))
                .unwrap_or_default();

            if let Ok(data_dir) = config::default_data_dir() {
                gate::maybe_record_from_prompt(
                    &data_dir,
                    &meta.prompt,
                    meta.session_id.as_deref(),
                    &config.gate,
                );
            }

            let modes = mode::load_modes(&config.resolve_modes_dirs(modes_dirs)?)?;
            let matched = mode::matching(&meta.prompt, &modes);
            if matched.is_empty() {
                return Ok(String::new());
            }
            if let Ok(data_dir) = config::default_data_dir() {
                log::append_attach(
                    &data_dir.join(log::ATTACH_LOG),
                    agent.as_str(),
                    &meta,
                    &matched,
                );
            }
            Ok(claude_code::encode(&matched))
        }
    }
}
