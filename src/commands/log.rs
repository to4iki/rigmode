use crate::config;
use crate::log;

pub fn execute(mode: Option<String>, limit: Option<usize>) -> anyhow::Result<()> {
    let path = config::default_data_dir()?.join(log::ATTACH_LOG);
    let records = log::list_attaches(&path, mode.as_deref(), limit);

    if records.is_empty() {
        eprintln!("No attach records in {}", path.display());
        return Ok(());
    }

    for r in records {
        println!(
            "{}\t{}\t{}\t{}",
            r.ts,
            r.modes.join(","),
            r.cwd.as_deref().unwrap_or("-"),
            r.session_id.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}
