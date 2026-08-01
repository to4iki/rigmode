use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "rigmode",
    version,
    about = "Attach work modes to AI coding agent prompts"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Match a prompt against modes and print agent-specific context (always exits 0)
    Attach {
        agent: Agent,
        /// Override modes directories (repeatable)
        #[arg(long = "modes-dir")]
        modes_dirs: Vec<std::path::PathBuf>,
    },

    /// Manage agent hooks
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Validate modes and hook registration
    Check {
        #[arg(long = "modes-dir")]
        modes_dirs: Vec<std::path::PathBuf>,
    },

    /// Show which mode would attach for a prompt
    Explain {
        /// Prompt text to match
        prompt: String,
        #[arg(long = "modes-dir")]
        modes_dirs: Vec<std::path::PathBuf>,
    },

    /// List recorded mode attaches (newest first)
    Log {
        /// Filter by attached mode name
        #[arg(long)]
        mode: Option<String>,
        /// Max rows to print
        #[arg(long)]
        limit: Option<usize>,
    },

    /// List recorded interventions (newest first)
    Gate {
        /// Filter by mode name
        #[arg(long)]
        mode: Option<String>,
        /// Max rows to print
        #[arg(long)]
        limit: Option<usize>,
    },
}

#[derive(Debug, Subcommand)]
pub enum HookAction {
    /// Register the attach hook for an agent
    Install {
        agent: Agent,
        /// Allow registering a binary under target/
        #[arg(long)]
        force: bool,
    },
    /// Remove the attach hook for an agent
    Uninstall { agent: Agent },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Agent {
    #[value(name = "claude-code")]
    ClaudeCode,
}

impl Agent {
    pub fn as_str(self) -> &'static str {
        match self {
            Agent::ClaudeCode => "claude-code",
        }
    }
}
