use std::env;
use std::path::Path;

use anyhow::{Result, bail};

use crate::adapters::claude_code;
use crate::cli::Agent;
use crate::commands::check;
use crate::config::Config;

pub fn install(agent: Agent, force: bool, config: &Config) -> Result<()> {
    match agent {
        Agent::ClaudeCode => {
            let binary = env::current_exe()?;
            let binary = binary.canonicalize().unwrap_or(binary);
            if !force && is_under_target(&binary) {
                bail!(
                    "refusing to register a binary under target/ ({}). \
                     Install with `cargo install --path .` or pass --force",
                    binary.display()
                );
            }
            let settings = claude_code::settings_path();
            claude_code::install_hook(&settings, &binary)?;
            println!(
                "UserPromptSubmit registered in {}\n  command: {}\n  args: [attach, claude-code]",
                settings.display(),
                binary.display()
            );
            // Surface mode/hook issues without failing install.
            let _ = check::execute(Vec::new(), config);
            Ok(())
        }
    }
}

pub fn uninstall(agent: Agent) -> Result<()> {
    match agent {
        Agent::ClaudeCode => {
            let settings = claude_code::settings_path();
            if claude_code::uninstall_hook(&settings)? {
                println!("UserPromptSubmit removed from {}", settings.display());
            } else {
                println!("nothing to remove in {}", settings.display());
            }
            Ok(())
        }
    }
}

fn is_under_target(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "target")
}
