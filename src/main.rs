use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command, HookAction};

mod adapters;
mod cli;
mod commands;
mod config;
mod gate;
mod log;
mod mode;
mod prompt;

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<ExitCode> {
    match command {
        Command::Attach { agent, modes_dirs } => {
            return Ok(commands::attach::execute(agent, modes_dirs));
        }
        Command::Hook { action } => match action {
            HookAction::Install { agent, force } => {
                commands::hook::install(agent, force, &load_config()?)?;
            }
            HookAction::Uninstall { agent } => commands::hook::uninstall(agent)?,
        },
        Command::Check { modes_dirs } => commands::check::execute(modes_dirs, &load_config()?)?,
        Command::Log { mode, limit } => commands::log::execute(mode, limit)?,
        Command::Gate { mode, limit } => commands::gate::execute(mode, limit)?,
    }
    Ok(ExitCode::SUCCESS)
}

fn load_config() -> Result<config::Config> {
    config::load_config(&config::default_config_path()?)
}
