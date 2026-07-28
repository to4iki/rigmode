use std::path::PathBuf;

use anyhow::Result;

use crate::commands::attach;
use crate::config::Config;
use crate::mode;

pub fn execute(prompt: &str, modes_dirs: Vec<PathBuf>, config: &Config) -> Result<()> {
    let dirs = attach::modes_dirs_for(modes_dirs, config)?;
    let modes = mode::load_modes(&dirs)?;

    match mode::select(prompt, &modes) {
        None => {
            println!("no mode matched");
            println!("prompt: {prompt}");
            println!("modes searched: {}", modes.len());
        }
        Some(sel) => {
            println!("chosen: {} (priority {})", sel.chosen.name, sel.chosen.priority);
            println!("path: {}", sel.chosen.path.display());
            if sel.matched_names.len() > 1 {
                println!("also matched:");
                for name in &sel.matched_names[1..] {
                    let priority = modes
                        .iter()
                        .find(|m| &m.name == name)
                        .map(|m| m.priority)
                        .unwrap_or(0);
                    println!("  - {name} (priority {priority})");
                }
                println!("reason: highest priority, then name ascending");
            } else {
                println!("reason: sole match");
            }
        }
    }
    Ok(())
}
