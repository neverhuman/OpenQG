mod payload;

use crate::util::write_generated_json;
use anyhow::Result;
use std::path::Path;
use std::time::Instant;

use super::findings::SecurityScan;

pub fn write(output: &Path, scan: &SecurityScan, start: Instant) -> Result<()> {
    let report = payload::build(scan, start);
    write_generated_json(output, "openqg-bench", "just security", &report)?;
    Ok(())
}
