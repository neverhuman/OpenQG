use crate::{ObservableRecord, PredictionRecord};
use anyhow::{bail, Result};

use super::ensure_non_empty;

pub fn validate_observable_record(record: &ObservableRecord) -> Result<()> {
    ensure_non_empty("observable_id", &record.observable_id)?;
    ensure_non_empty("kind", &record.kind)?;
    ensure_non_empty("unit", &record.unit)?;
    if !record.value.is_finite() || !record.uncertainty.is_finite() || record.uncertainty < 0.0 {
        bail!("observable values must be finite with non-negative uncertainty");
    }
    Ok(())
}

pub fn validate_prediction_record(record: &PredictionRecord) -> Result<()> {
    ensure_non_empty("observable_id", &record.observable_id)?;
    ensure_non_empty("unit", &record.unit)?;
    if !record.value.is_finite() || !record.uncertainty.is_finite() || record.uncertainty < 0.0 {
        bail!("prediction values must be finite with non-negative uncertainty");
    }
    Ok(())
}
