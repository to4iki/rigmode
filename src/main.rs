use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;
use rigmode::cli::{Cli, Command, HookAction};
use rigmode::{commands, config};

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Command::Attach { agent, modes_dirs } = cli.command {
        return commands::attach::execute(agent, modes_dirs);
    }

    match run(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<()> {
    let config_path = config::default_config_path()?;
    let config = config::load_config(&config_path)?;

    match command {
        Command::Attach { .. } => unreachable!("handled in main"),
        Command::Hook { action } => match action {
            HookAction::Install { agent, force } => {
                commands::hook::install(agent, force, &config)
            }
            HookAction::Uninstall { agent } => commands::hook::uninstall(agent),
        },
        Command::Check { modes_dirs } => commands::check::execute(modes_dirs, &config),
        Command::Explain { prompt, modes_dirs } => {
            commands::explain::execute(&prompt, modes_dirs, &config)
        }
    }
}
