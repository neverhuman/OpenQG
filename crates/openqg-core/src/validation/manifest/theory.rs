use crate::TheoryManifest;
use anyhow::{bail, Result};

use super::{ensure_no_black_box_text, ensure_non_empty, reject_raw_path};

pub fn validate_theory_manifest(manifest: &TheoryManifest) -> Result<()> {
    ensure_non_empty("theory id", &manifest.id)?;
    ensure_non_empty("theory name", &manifest.name)?;
    ensure_non_empty("status", &manifest.status)?;
    ensure_non_empty("benchmark_suite", &manifest.benchmark_suite)?;
    ensure_non_empty("description", &manifest.description)?;
    ensure_no_black_box_text("description", &manifest.description)?;
    if manifest.parameters.is_empty() {
        bail!("parameters must not be empty");
    }
    if manifest.observables.is_empty() {
        bail!("observables must not be empty");
    }
    if manifest.citations.is_empty() {
        bail!("citations must not be empty");
    }

    for parameter in &manifest.parameters {
        ensure_non_empty("parameter.name", &parameter.name)?;
        ensure_non_empty("parameter.symbol", &parameter.symbol)?;
        ensure_non_empty("parameter.unit", &parameter.unit)?;
        ensure_non_empty("parameter.physical_meaning", &parameter.physical_meaning)?;
        ensure_non_empty(
            "parameter.prior_or_fixed_value",
            &parameter.prior_or_fixed_value,
        )?;
        ensure_non_empty(
            "parameter.source_or_free_reason",
            &parameter.source_or_free_reason,
        )?;
        ensure_no_black_box_text("parameter.physical_meaning", &parameter.physical_meaning)?;
        if parameter.unit.trim() == "dimensionless" {
            ensure_no_black_box_text("dimensionless parameter", &parameter.name)?;
        }
    }

    for observable in &manifest.observables {
        ensure_non_empty("observable", observable)?;
    }

    ensure_non_empty("adapter.command", &manifest.adapter.command)?;
    if !manifest.adapter.command.contains("openqg-theory predict")
        && !manifest.adapter.command.contains("python")
        && !manifest.adapter.command.contains("cargo")
    {
        bail!("adapter.command must be CLI-compatible");
    }

    if let Some(predictions_path) = &manifest.adapter.predictions_path {
        reject_raw_path("adapter.predictions_path", predictions_path)?;
    }

    Ok(())
}
