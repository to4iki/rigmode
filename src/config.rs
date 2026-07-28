use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub modes_dirs: Vec<PathBuf>,
}

impl Config {
    /// Resolved modes directories. Falls back to the default modes dir when empty.
    pub fn resolved_modes_dirs(&self) -> Result<Vec<PathBuf>> {
        if self.modes_dirs.is_empty() {
            Ok(vec![default_modes_dir()?])
        } else {
            Ok(self.modes_dirs.clone())
        }
    }
}

/// Expand a leading `~` or `~/` to the given home directory.
pub fn expand_tilde(path: &Path, home: Option<&Path>) -> PathBuf {
    let Some(home) = home else {
        return path.to_path_buf();
    };
    match path.to_str() {
        Some("~") => home.to_path_buf(),
        Some(s) => match s.strip_prefix("~/") {
            Some(rest) => home.join(rest),
            None => path.to_path_buf(),
        },
        None => path.to_path_buf(),
    }
}

fn config_path_from(home: Option<&Path>, xdg_config_home: Option<&Path>) -> Option<PathBuf> {
    let base = xdg_config_home
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".config")))?;
    Some(base.join("rigmode").join("config.toml"))
}

fn data_dir_from(home: Option<&Path>, xdg_data_home: Option<&Path>) -> Option<PathBuf> {
    let base = xdg_data_home
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local").join("share")))?;
    Some(base.join("rigmode"))
}

/// Prefers `$XDG_CONFIG_HOME/rigmode/config.toml`, falling back to `~/.config/rigmode/config.toml`.
pub fn default_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir();
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    config_path_from(home.as_deref(), xdg.as_deref())
        .context("Could not determine config directory")
}

/// Prefers `$XDG_CONFIG_HOME/rigmode/modes`, falling back to `~/.config/rigmode/modes`.
pub fn default_modes_dir() -> Result<PathBuf> {
    let home = dirs::home_dir();
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let base = xdg
        .or_else(|| home.map(|h| h.join(".config")))
        .context("Could not determine config directory")?;
    Ok(base.join("rigmode").join("modes"))
}

/// Prefers `$XDG_DATA_HOME/rigmode`, falling back to `~/.local/share/rigmode`.
pub fn default_data_dir() -> Result<PathBuf> {
    let home = dirs::home_dir();
    let xdg = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from);
    data_dir_from(home.as_deref(), xdg.as_deref()).context("Could not determine data directory")
}

pub fn load_config(path: &Path) -> Result<Config> {
    load_config_with_home(path, dirs::home_dir().as_deref())
}

fn load_config_with_home(path: &Path, home: Option<&Path>) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let mut config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    config.modes_dirs = config
        .modes_dirs
        .into_iter()
        .map(|p| expand_tilde(&p, home))
        .collect();

    Ok(config)
}
