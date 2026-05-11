use anyhow::Result;
use std::path::Path;
use std::time::Instant;

mod findings;
mod report;

pub fn scan(output: &Path) -> Result<()> {
    let start = Instant::now();
    let scan = findings::collect(Path::new("."))?;
    report::write(output, &scan, start)?;
    println!(
        "wrote security evidence to {} ({})",
        output.display(),
        scan.status
    );
    Ok(())
}
