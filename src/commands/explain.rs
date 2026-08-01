use std::path::PathBuf;

use anyhow::Result;

use crate::commands::attach;
use crate::config::Config;
use crate::mode;

pub fn execute(prompt: &str, modes_dirs: Vec<PathBuf>, config: &Config) -> Result<()> {
    let dirs = attach::modes_dirs_for(modes_dirs, config)?;
    let modes = mode::load_modes(&dirs)?;

    let matched = mode::matching(prompt, &modes);
    if matched.is_empty() {
        println!("no mode matched");
        println!("prompt: {prompt}");
        println!("modes searched: {}", modes.len());
    } else {
        let names: Vec<&str> = matched.iter().map(|m| m.name.as_str()).collect();
        println!("attached: {}", names.join(", "));
        for m in &matched {
            println!("  - {} ({})", m.name, m.path.display());
        }
        println!("reason: every matching mode attaches, in load order");
    }
    Ok(())
}
