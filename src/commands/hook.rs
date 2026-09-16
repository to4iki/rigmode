use std::env;
use std::path::Path;

use anyhow::{Result, bail};

use crate::adapters;
use crate::cli::Agent;
use crate::commands::check;
use crate::config::Config;

pub fn install(agent: Agent, force: bool, config: &Config) -> Result<()> {
    let binary = env::current_exe()?;
    let binary = binary.canonicalize().unwrap_or(binary);
    if !force && is_under_target(&binary) {
        bail!(
            "refusing to register a binary under target/ ({}). \
             Install with `cargo install --path .` or pass --force",
            binary.display()
        );
    }
    let hooks = adapters::hooks(agent);
    hooks.install(&binary)?;
    println!(
        "UserPromptSubmit registered in {}\n  command: {} attach {}",
        hooks.path.display(),
        binary.display(),
        agent.as_str()
    );
    if matches!(agent, Agent::Codex) {
        println!("Codex skips untrusted hooks: run /hooks inside Codex and trust this entry.");
    }
    // Surface mode/hook issues without failing install.
    let _ = check::execute(Vec::new(), config);
    Ok(())
}

pub fn uninstall(agent: Agent) -> Result<()> {
    let hooks = adapters::hooks(agent);
    if hooks.uninstall()? {
        println!("UserPromptSubmit removed from {}", hooks.path.display());
    } else {
        println!("nothing to remove in {}", hooks.path.display());
    }
    Ok(())
}

fn is_under_target(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "target")
}
