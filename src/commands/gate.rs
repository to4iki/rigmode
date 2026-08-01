use crate::config;
use crate::gate;

pub fn execute(mode: Option<String>, limit: Option<usize>) -> anyhow::Result<()> {
    let path = config::default_data_dir()?.join(gate::GATES_LOG);
    let records = gate::list_gates(&path, mode.as_deref(), limit);

    if records.is_empty() {
        eprintln!("No gate records in {}", path.display());
        return Ok(());
    }

    for r in records {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            r.ts,
            r.modes.join(","),
            r.marker,
            r.note,
            r.session_id
        );
    }
    Ok(())
}
