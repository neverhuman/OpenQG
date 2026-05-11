use crate::util::{generated_at, read_jsonl};
use anyhow::{bail, Result};
use openqg_core::{
    validate_observable_record, validate_prediction_record, ObservableRecord, PredictionRecord,
};
use openqg_data::{validate_registry, write_data_lock};
use std::path::Path;

pub fn lock(root: &Path, output: &Path) -> Result<()> {
    let lock = write_data_lock(root, output, generated_at())?;
    println!(
        "wrote {} entries to {}",
        lock.entries.len(),
        output.display()
    );
    Ok(())
}

pub fn verify(root: &Path, output: &Path) -> Result<()> {
    let manifests = validate_registry(root)?;
    let _lock = write_data_lock(root, output, generated_at())?;
    println!("validated {} dataset manifests", manifests.len());
    Ok(())
}

pub fn smoke(observables: &Path, predictions: &Path) -> Result<()> {
    let observables: Vec<ObservableRecord> = read_jsonl(observables)?;
    let predictions: Vec<PredictionRecord> = read_jsonl(predictions)?;
    for observable in &observables {
        validate_observable_record(observable)?;
    }
    for prediction in &predictions {
        validate_prediction_record(prediction)?;
    }
    if observables.is_empty() || predictions.is_empty() {
        bail!("smoke fixtures must not be empty");
    }
    println!(
        "validated {} observables and {} predictions",
        observables.len(),
        predictions.len()
    );
    Ok(())
}
