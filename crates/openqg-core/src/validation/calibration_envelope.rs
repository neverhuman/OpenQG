//! V8 Phase 1 (#7): CalibrationEnvelope — residual comparison between the in-repo fitting
//! formulae (T1Emulator) and a Boltzmann solver (T2Boltzmann).
//!
//! The envelope quantifies how much the fitting formulae drift from the full Boltzmann solution
//! as a function of parameter space position. Where the drift exceeds a threshold, the T1 score
//! is flagged as `calibration_suspect` and must not be used for promotion-grade claims.
//!
//! **Solver integration**: this module contains the Rust types and checks; actually running
//! the Boltzmann solver requires `cosmology/subprocess.rs` and an external binary.

use serde::{Deserialize, Serialize};

/// The residual comparison between two forward-model predictions on the same observables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionResidual {
    /// Observable ID (e.g. "s8", "cmb_R", "dv_over_rd@0.295").
    pub observable_id: String,
    /// Value from the reference (higher-fidelity) model.
    pub reference_value: f64,
    /// Value from the fitting formula (lower-fidelity) model.
    pub formula_value: f64,
    /// Fractional residual = (formula - reference) / reference.
    pub fractional_residual: f64,
    /// Whether this residual exceeds the calibration tolerance threshold.
    pub exceeds_threshold: bool,
}

impl PredictionResidual {
    /// Compute a residual between reference and formula values.
    ///
    /// `threshold`: fractional residual above which the observable is flagged.
    pub fn compute(
        observable_id: impl Into<String>,
        reference: f64,
        formula: f64,
        threshold: f64,
    ) -> Self {
        let fractional = if reference.abs() < 1e-30 {
            0.0
        } else {
            (formula - reference) / reference.abs()
        };
        PredictionResidual {
            observable_id: observable_id.into(),
            reference_value: reference,
            formula_value: formula,
            fractional_residual: fractional,
            exceeds_threshold: fractional.abs() > threshold,
        }
    }
}

/// The full calibration envelope for one theory at one parameter-space point.
///
/// Produced by running both the T1Emulator (fitting formula) and the T2Boltzmann (full solver)
/// on the same observable set at the same parameter values, then comparing the results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationEnvelope {
    /// The theory whose parameters were used.
    pub theory_id: String,

    /// Per-observable residuals.
    pub residuals: Vec<PredictionResidual>,

    /// Fractional residual threshold used for flagging. Default 0.01 (1%).
    pub threshold: f64,

    /// True when ANY observable exceeds the threshold. This envelope must not be used to
    /// report promotion-grade results at this parameter-space point.
    pub calibration_suspect: bool,
}

impl CalibrationEnvelope {
    /// Build an envelope from a list of (observable_id, reference, formula) triples.
    pub fn build(
        theory_id: impl Into<String>,
        triples: &[(&str, f64, f64)],
        threshold: f64,
    ) -> Self {
        let residuals: Vec<_> = triples
            .iter()
            .map(|(id, reference, formula)| {
                PredictionResidual::compute(*id, *reference, *formula, threshold)
            })
            .collect();
        let calibration_suspect = residuals.iter().any(|r| r.exceeds_threshold);
        CalibrationEnvelope {
            theory_id: theory_id.into(),
            residuals,
            threshold,
            calibration_suspect,
        }
    }

    /// Root-mean-square fractional residual across all observables.
    pub fn rms_residual(&self) -> f64 {
        if self.residuals.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = self
            .residuals
            .iter()
            .map(|r| r.fractional_residual.powi(2))
            .sum();
        (sum_sq / self.residuals.len() as f64).sqrt()
    }

    /// Maximum absolute fractional residual across all observables.
    pub fn max_residual(&self) -> f64 {
        self.residuals
            .iter()
            .map(|r| r.fractional_residual.abs())
            .fold(0.0_f64, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn residual_computed_and_flagged_correctly() {
        let r = PredictionResidual::compute("s8", 0.800, 0.810, 0.01);
        // fractional = (0.810 - 0.800) / 0.800 = 0.0125; exceeds 0.01
        assert!((r.fractional_residual - 0.0125).abs() < 1e-10);
        assert!(r.exceeds_threshold);
    }

    #[test]
    fn residual_not_flagged_within_tolerance() {
        let r = PredictionResidual::compute("h0", 67.4, 67.5, 0.01);
        // fractional ≈ 0.00148; within 0.01
        assert!(r.fractional_residual.abs() < 0.01);
        assert!(!r.exceeds_threshold);
    }

    #[test]
    fn zero_reference_gives_zero_residual() {
        let r = PredictionResidual::compute("aux", 0.0, 0.001, 0.01);
        assert_eq!(r.fractional_residual, 0.0);
    }

    #[test]
    fn calibration_suspect_when_any_exceeds_threshold() {
        let env = CalibrationEnvelope::build(
            "ndgp",
            &[
                ("h0", 67.4, 67.5),   // ok (0.15%)
                ("s8", 0.800, 0.815), // suspect (1.875%)
            ],
            0.01,
        );
        assert!(env.calibration_suspect);
        assert_eq!(env.residuals.len(), 2);
    }

    #[test]
    fn not_suspect_when_all_within_tolerance() {
        let env = CalibrationEnvelope::build(
            "gr-lcdm",
            &[("h0", 67.4, 67.41), ("s8", 0.800, 0.801)],
            0.01,
        );
        assert!(!env.calibration_suspect);
    }

    #[test]
    fn rms_residual_correct() {
        // residuals: 0.01 and 0.01 → rms = 0.01
        let env = CalibrationEnvelope::build("test", &[("a", 1.0, 1.01), ("b", 2.0, 2.02)], 0.05);
        assert!((env.rms_residual() - 0.01).abs() < 1e-10);
    }

    #[test]
    fn max_residual_correct() {
        let env = CalibrationEnvelope::build("test", &[("a", 1.0, 1.01), ("b", 2.0, 2.10)], 0.10);
        assert!((env.max_residual() - 0.05).abs() < 1e-10);
    }

    #[test]
    fn empty_residuals_gives_zero_rms() {
        let env = CalibrationEnvelope::build("empty", &[], 0.01);
        assert_eq!(env.rms_residual(), 0.0);
        assert!(!env.calibration_suspect);
    }
}
