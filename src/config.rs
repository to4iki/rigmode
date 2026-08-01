use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub modes_dirs: Vec<PathBuf>,
    pub gate: GateConfig,
}

/// Words that mark a prompt as a human intervention (a rejection of the
/// agent's work). An empty list (the default) disables recording.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct GateConfig {
    pub markers: Vec<String>,
}

impl Config {
    /// CLI override dirs win; else config entries; else `<config>/rigmode/modes`.
    pub fn resolve_modes_dirs(&self, overrides: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        if !overrides.is_empty() {
            Ok(overrides)
        } else if self.modes_dirs.is_empty() {
            Ok(vec![config_base_dir()?.join("modes")])
        } else {
            Ok(self.modes_dirs.clone())
        }
    }
}

/// Expand a leading `~` or `~/` to the home directory.
fn expand_tilde(path: &Path) -> PathBuf {
    let Some(home) = dirs::home_dir() else {
        return path.to_path_buf();
    };
    match path.to_str() {
        Some("~") => home,
        Some(s) => match s.strip_prefix("~/") {
            Some(rest) => home.join(rest),
            None => path.to_path_buf(),
        },
        None => path.to_path_buf(),
    }
}

/// `$XDG_CONFIG_HOME/rigmode`, falling back to `~/.config/rigmode`.
fn config_base_dir() -> Result<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
        .map(|base| base.join("rigmode"))
        .context("Could not determine config directory")
}

pub fn default_config_path() -> Result<PathBuf> {
    Ok(config_base_dir()?.join("config.toml"))
}

/// `$XDG_DATA_HOME/rigmode`, falling back to `~/.local/share/rigmode`.
pub fn default_data_dir() -> Result<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
        .map(|base| base.join("rigmode"))
        .context("Could not determine data directory")
}

/// Missing file is the zero-config default, not an error.
pub fn load_config(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let mut config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    config.modes_dirs = config.modes_dirs.iter().map(|p| expand_tilde(p)).collect();

    Ok(config)
}
