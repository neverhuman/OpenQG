use crate::util::write_generated_json;
use anyhow::{bail, Result};
use openqg_core::preview_zyal_document;
use std::path::Path;

pub fn validate(root: &Path, output: &Path) -> Result<()> {
    openqg_core::validate_zyal_layout(Path::new("."))?;

    let mut previews = Vec::new();
    for path in openqg_core::walk_zyal_files(root)? {
        let preview = preview_zyal_document(&path)?;
        previews.push(preview);
    }
    if previews.is_empty() {
        bail!("no ZYAL runbooks found under {}", root.display());
    }
    write_generated_json(output, "openqg-bench", "just zyal-validate", &previews)?;
    println!("validated {} zyal runbooks", previews.len());
    Ok(())
}
