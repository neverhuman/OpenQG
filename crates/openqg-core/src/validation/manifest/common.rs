use anyhow::{bail, Result};

pub fn ensure_non_empty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(())
}

pub fn ensure_no_black_box_text(label: &str, value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    if lower.contains("black box")
        || lower.contains("gray box")
        || lower.contains("grey box")
        || lower.contains("latent")
        || lower.contains("fit until")
    {
        bail!("{label} contains black-box language");
    }
    Ok(())
}

pub fn ensure_http_url(label: &str, value: &str) -> Result<()> {
    if !(value.starts_with("http://") || value.starts_with("https://")) {
        bail!("{label} must be an http(s) URL");
    }
    Ok(())
}

pub fn reject_raw_path(label: &str, path: &str) -> Result<()> {
    let lower = path.to_ascii_lowercase();
    if lower.contains("/raw/") || lower.contains("data/raw") || lower.contains("raw-data") {
        bail!("{label} points at raw data");
    }
    Ok(())
}
